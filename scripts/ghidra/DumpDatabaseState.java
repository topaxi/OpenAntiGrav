/* Dump everything a Ghidra database holds that names.tsv does not carry.
 *
 * Run inside Ghidra (Script Manager, or the MCP bridge's run_script_inline) with
 * any program open; it walks every program in the project itself rather than
 * acting on the current one. Writes raw per-program TSVs to the directory named
 * by the first argument, which scripts/capture-ghidra-state.py then filters into
 * docs/ghidra/captures/.
 *
 * Two traps this deliberately avoids:
 *
 * A program is opened by its DomainFile pathname, never by its display name.
 * Four programs in this project are called `BOOT.BIN`, and an old database and
 * its fresh reimport (`BOOT.BIN` and `BOOT.BIN.0`) also share an executable
 * path, so neither the name nor the path distinguishes them - only the pathname
 * and the version number do. switch_program's success reply is not trustworthy
 * for these; see scripts/apply-ghidra-names.py's Bridge.switch_program.
 *
 * Nothing here reads program memory. The dump is addresses, symbol names, type
 * names and comment prose - all of it written by this project or by Ghidra, none
 * of it bytes out of the executable, so ADR-0006 is satisfied by construction
 * rather than by review.
 *
 * @category OpenAntiGrav
 */

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;

import ghidra.app.script.GhidraScript;
import ghidra.framework.model.DomainFile;
import ghidra.framework.model.DomainFolder;
import ghidra.program.model.address.Address;
import ghidra.program.model.address.AddressIterator;
import ghidra.program.model.data.Composite;
import ghidra.program.model.data.DataType;
import ghidra.program.model.listing.CodeUnit;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Listing;
import ghidra.program.model.listing.Parameter;
import ghidra.program.model.listing.Program;
import ghidra.program.model.listing.Variable;
import ghidra.program.model.symbol.SourceType;
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.symbol.SymbolType;

public class DumpDatabaseState extends GhidraScript {

    /* Decompiler-generated variable names. A name matching one of these is not a
     * claim anyone made, so it is not part of the capture. */
    private static final String GENERATED_VAR =
        "^(local_|param_|in_|unaff_|extraout_|register0x).*";
    private static final String GENERATED_VAR_TYPED = "^[a-z]{0,3}[A-Za-z]?Var[0-9]+$";

    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        String out = args.length > 0 ? args[0] : "/tmp/ghidra-capture/";
        if (!out.endsWith("/")) {
            out = out + "/";
        }
        new File(out).mkdirs();

        PrintWriter summary = new PrintWriter(new FileWriter(out + "survey.txt"));
        for (DomainFile df : programs()) {
            Program p = (Program) df.getDomainObject(this, false, false, monitor);
            try {
                dump(p, df, out, summary);
            } finally {
                p.release(this);
            }
        }
        summary.close();
        println("wrote " + out);
    }

    private List<DomainFile> programs() {
        List<DomainFile> files = new ArrayList<>();
        ArrayDeque<DomainFolder> queue = new ArrayDeque<>();
        queue.add(state.getProject().getProjectData().getRootFolder());
        while (!queue.isEmpty()) {
            DomainFolder folder = queue.poll();
            for (DomainFolder sub : folder.getFolders()) {
                queue.add(sub);
            }
            for (DomainFile file : folder.getFiles()) {
                if ("Program".equals(file.getContentType())) {
                    files.add(file);
                }
            }
        }
        return files;
    }

    private void dump(Program p, DomainFile df, String out, PrintWriter summary) throws Exception {
        String tag = df.getPathname().substring(1).replace('/', '_');
        PrintWriter funcs = new PrintWriter(new FileWriter(out + tag + ".funcs.tsv"));
        PrintWriter sigs = new PrintWriter(new FileWriter(out + tag + ".sigs.tsv"));
        PrintWriter vars = new PrintWriter(new FileWriter(out + tag + ".vars.tsv"));
        PrintWriter comments = new PrintWriter(new FileWriter(out + tag + ".comments.tsv"));
        PrintWriter labels = new PrintWriter(new FileWriter(out + tag + ".labels.tsv"));

        int total = 0;
        int defaultNamed = 0;
        int signatures = 0;
        int namedParams = 0;
        int namedLocals = 0;

        for (Function fn : p.getFunctionManager().getFunctions(true)) {
            total++;
            String name = fn.getName();
            String addr = fn.getEntryPoint().toString();
            if (name.startsWith("FUN_")) {
                defaultNamed++;
            }
            if (!name.startsWith("FUN_") && !name.startsWith("LAB_")
                    && !name.startsWith("SUB_") && !name.startsWith(".opd.FUN_")) {
                funcs.println(addr + "\t" + name + "\t" + fn.getSymbol().getSource());
            }
            if (fn.getSignatureSource() != SourceType.DEFAULT) {
                signatures++;
                sigs.println(addr + "\t" + name + "\t" + fn.getSignatureSource() + "\t"
                    + clean(fn.getSignature().getPrototypeString(true)));
            }
            String plate = fn.getComment();
            if (plate != null && !plate.isEmpty()) {
                comments.println(addr + "\tPLATE\t" + name + "\t" + clean(plate));
            }
            for (Variable v : fn.getAllVariables()) {
                String vn = v.getName();
                if (v.getSource() == SourceType.DEFAULT || vn == null) {
                    continue;
                }
                if (vn.matches(GENERATED_VAR) || vn.matches(GENERATED_VAR_TYPED)) {
                    continue;
                }
                if (v instanceof Parameter) {
                    namedParams++;
                } else {
                    namedLocals++;
                }
                vars.println(addr + "\t" + name + "\t" + (v instanceof Parameter ? "param" : "local")
                    + "\t" + vn + "\t" + v.getDataType().getName() + "\t" + v.getVariableStorage());
            }
        }

        /* Composites, minus every category a loader or a header import owns. What
         * survives is a structure somebody recovered - which, project-wide on
         * 2026-09-07, was none. */
        int composites = 0;
        StringBuilder names = new StringBuilder();
        Iterator<DataType> it = p.getDataTypeManager().getAllDataTypes();
        while (it.hasNext()) {
            DataType d = it.next();
            String cat = d.getCategoryPath().getPath();
            boolean builtin = cat.startsWith("/ELF") || cat.startsWith("/stddef")
                || cat.startsWith("/DWARF") || cat.startsWith("/PE") || cat.startsWith("/SCE")
                || cat.startsWith("/PS3") || cat.startsWith("/demangler");
            boolean interesting = d instanceof Composite || d instanceof ghidra.program.model.data.Enum;
            if (interesting && !builtin) {
                composites++;
                names.append(" [" + cat + ":" + d.getName() + ":" + d.getLength() + "]");
            }
        }

        int userLabels = 0;
        for (Symbol s : p.getSymbolTable().getAllSymbols(false)) {
            if (s.getSource() == SourceType.USER_DEFINED && s.getSymbolType() != SymbolType.FUNCTION) {
                userLabels++;
                labels.println(s.getAddress() + "\t" + s.getSymbolType() + "\t" + s.getName()
                    + "\t" + s.getParentNamespace().getName());
            }
        }

        int plates = 0;
        Listing listing = p.getListing();
        AddressIterator ai = listing.getCommentAddressIterator(p.getMemory(), true);
        while (ai.hasNext()) {
            Address a = ai.next();
            String c = listing.getComment(CodeUnit.PLATE_COMMENT, a);
            if (c != null) {
                plates++;
                comments.println(a + "\tPLATE_ANY\t\t" + clean(c));
            }
        }

        summary.println(df.getPathname() + " ver=" + df.getVersion()
            + " exe=" + p.getExecutablePath() + " base=" + p.getImageBase()
            + " funcs=" + total + " defaultNamed=" + defaultNamed
            + " customSig=" + signatures + " namedParams=" + namedParams
            + " namedLocals=" + namedLocals + " userTypes=" + composites
            + " userLabels=" + userLabels + " plateAny=" + plates);
        if (composites > 0) {
            summary.println("   TYPES:" + names);
        }

        funcs.close();
        sigs.close();
        vars.close();
        comments.close();
        labels.close();
    }

    /* The capture is TSV, one row per line. A tab or a newline inside a comment
     * would silently produce a ragged row that check-ghidra-captures.py then
     * rejects, so both are escaped here rather than at the far end. */
    private static String clean(String s) {
        return s.replace("\t", " ").replace("\r", "").replace("\n", "\\n");
    }
}
