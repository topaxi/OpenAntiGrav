/* Follow-up to PadImporterVtableDiff.java: the raw vtable slot values are OPD
 * (function descriptor) addresses, not code addresses directly - the first
 * pass's describe() found no Function at any of them because it looked up
 * the wrong address. This resolves each slot through the symbol at that
 * address (named ".opd.FUN_<hex>" or a real name for an already-recovered
 * function) and, where that fails, dereferences the OPD's own first word.
 *
 * Read-only: no renames, no comments, no database writes.
 *
 * @category OpenAntiGrav
 */

import java.io.FileWriter;
import java.io.PrintWriter;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.symbol.SymbolTable;

public class PadImporterVtableDiff2 extends GhidraScript {

    private static final long WEAPON_VTABLE = 0x0086a730L;
    private static final long SPEEDUP_VTABLE = 0x0086a780L;
    private static final int SLOTS = 17;

    @Override
    public void run() throws Exception {
        String out = getScriptArgs().length > 0 ? getScriptArgs()[0]
            : "/tmp/pad-importer-vtable-diff2.txt";
        PrintWriter w = new PrintWriter(new FileWriter(out));

        long[] weapon = readVtable(WEAPON_VTABLE, w, "weapon");
        long[] speedup = readVtable(SPEEDUP_VTABLE, w, "speedup");

        w.println();
        w.println("# Diff, resolved");
        for (int i = 0; i < SLOTS; i++) {
            String wd = describe(weapon[i]);
            String sd = describe(speedup[i]);
            boolean differs = !wd.equals(sd);
            w.println("slot " + i + ": weapon=" + wd + " speedup=" + sd + (differs ? "  DIFFERS" : ""));
        }

        w.close();
        println("wrote " + out);
    }

    private long[] readVtable(long addr, PrintWriter w, String label) throws Exception {
        Memory mem = currentProgram.getMemory();
        long[] slots = new long[SLOTS];
        Address base = toAddr(addr);
        for (int i = 0; i < SLOTS; i++) {
            Address slotAddr = base.add((long) i * 4);
            long target = mem.getInt(slotAddr) & 0xffffffffL;
            slots[i] = target;
            w.println(label + " slot " + i + " @ " + slotAddr + " -> " + describe(target));
        }
        return slots;
    }

    private String describe(long target) {
        Address a = toAddr(target);

        Function fn = currentProgram.getFunctionManager().getFunctionAt(a);
        if (fn != null) {
            return hex(target) + " direct-func:" + fn.getName();
        }

        SymbolTable st = currentProgram.getSymbolTable();
        Symbol sym = st.getPrimarySymbol(a);
        if (sym != null) {
            String name = sym.getName();
            if (name.startsWith(".opd.")) {
                return hex(target) + " opd-name:" + name.substring(".opd.".length());
            }
            return hex(target) + " sym:" + name;
        }

        // Fall back: treat target as an OPD entry and dereference its first word.
        try {
            long codeAddr = currentProgram.getMemory().getInt(a) & 0xffffffffL;
            Address codeA = toAddr(codeAddr);
            Function codeFn = currentProgram.getFunctionManager().getFunctionAt(codeA);
            if (codeFn != null) {
                return hex(target) + " opd-deref:" + codeFn.getName() + "@" + hex(codeAddr);
            }
            return hex(target) + " opd-deref-unnamed:" + hex(codeAddr);
        } catch (Exception e) {
            return hex(target) + " (unresolved: " + e.getMessage() + ")";
        }
    }

    private static String hex(long v) {
        return "0x" + Long.toHexString(v);
    }
}
