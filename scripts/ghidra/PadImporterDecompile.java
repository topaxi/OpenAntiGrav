/* Decompile the Pad_Importer vtable slots that differ between WeaponPad_Importer
 * and SpeedupPad_Importer (found by PadImporterVtableDiff2.java), to read what
 * each one actually does rather than just diffing addresses.
 *
 * Read-only: decompiles only, no renames, no comments, no database writes.
 *
 * @category OpenAntiGrav
 */

import java.io.FileWriter;
import java.io.PrintWriter;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.util.task.ConsoleTaskMonitor;

public class PadImporterDecompile extends GhidraScript {

    private static final long[] TARGETS = {
        0x002bf238L, 0x002e0648L, // slot 0: weapon, speedup
        0x002e02b8L, 0x002dff40L, // slot 3
        0x002be9c0L, 0x002e0020L, // slot 5
        0x006b57d8L, 0x006b59f8L, // slot 12
        0x006b5818L, 0x006b5f78L, // slot 13
        0x006b5870L, 0x006b5fd8L, // slot 14
        0x006b4dd8L, 0x002e0590L, // slot 15
        0x002be978L, 0x006af9a0L, // slot 16
    };

    @Override
    public void run() throws Exception {
        String out = getScriptArgs().length > 0 ? getScriptArgs()[0] : "/tmp/pad-importer-decompile.txt";
        PrintWriter w = new PrintWriter(new FileWriter(out));

        DecompInterface decomp = new DecompInterface();
        decomp.openProgram(currentProgram);

        for (long addr : TARGETS) {
            Address a = toAddr(addr);
            Function fn = currentProgram.getFunctionManager().getFunctionAt(a);
            w.println("==== " + String.format("0x%08x", addr) + " "
                + (fn != null ? fn.getName() : "(no function)") + " ====");
            if (fn == null) {
                w.println("(no function defined here)");
                continue;
            }
            DecompileResults res = decomp.decompileFunction(fn, 60, new ConsoleTaskMonitor());
            if (res != null && res.decompileCompleted()) {
                w.println(res.getDecompiledFunction().getC());
            } else {
                w.println("(decompile failed: " + (res != null ? res.getErrorMessage() : "null result") + ")");
            }
            w.println();
        }

        decomp.dispose();
        w.close();
        println("wrote " + out);
    }
}
