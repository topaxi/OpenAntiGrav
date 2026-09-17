/* Dump raw words and any symbol name around the two Pad_Importer vtables to
 * find the real table boundary - PadImporterVtableDiff.java found a 0xc-byte
 * gap between the claimed 17-slot end of one and the start of the next,
 * which does not fit a clean back-to-back layout.
 *
 * Read-only.
 *
 * @category OpenAntiGrav
 */

import java.io.FileWriter;
import java.io.PrintWriter;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.symbol.SymbolTable;

public class PadImporterVtableRaw extends GhidraScript {
    @Override
    public void run() throws Exception {
        String out = getScriptArgs().length > 0 ? getScriptArgs()[0] : "/tmp/pad-vtable-raw.txt";
        PrintWriter w = new PrintWriter(new FileWriter(out));
        SymbolTable st = currentProgram.getSymbolTable();

        long start = 0x0086a700L;
        long end = 0x0086a800L;
        for (long addr = start; addr < end; addr += 4) {
            Address a = toAddr(addr);
            long v = currentProgram.getMemory().getInt(a) & 0xffffffffL;
            Symbol sym = st.getPrimarySymbol(a);
            String name = sym != null ? sym.getName() : "";
            w.println(String.format("%08x: %08x  %s", addr, v, name));
        }
        w.close();
        println("wrote " + out);
    }
}
