/* Read the constants WeaponPad_UpdateRefreshTimer (0x002e02b8) uses: the
 * refresh-timer floor, the keyframe table's six RGB triples, and the neutral
 * vector at 0x00aec2c0 read through TOC slot 0x008b431c while cooling down.
 *
 * Read-only.
 *
 * @category OpenAntiGrav
 */

import java.io.FileWriter;
import java.io.PrintWriter;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;

public class PadColourCycleConstants extends GhidraScript {
    @Override
    public void run() throws Exception {
        String out = getScriptArgs().length > 0 ? getScriptArgs()[0] : "/tmp/pad-colour-cycle-constants.txt";
        PrintWriter w = new PrintWriter(new FileWriter(out));

        w.println("DAT_008b42f8 (refresh-timer floor) = " + f(0x008b42f8L));
        w.println("DAT_008b4328 = " + f(0x008b4328L));
        w.println("DAT_008b4324 (dt scale?) = " + f(0x008b4324L));

        long tocSlot = 0x008b431cL;
        long tableAddr = readPtr(0x008b4320L);
        w.println();
        w.println("PTR_DAT_008b4320 (keyframe table) -> " + hex(tableAddr));
        for (int i = 0; i < 8; i++) {
            long base = tableAddr + (long) i * 0xc;
            w.println("  entry " + i + " @ " + hex(base) + ": "
                + f(base) + ", " + f(base + 4) + ", " + f(base + 8));
        }

        long neutralPtr = readPtr(tocSlot);
        w.println();
        w.println("PTR_DAT_008b431c (neutral/cooldown vector pointer) -> " + hex(neutralPtr));
        w.println("  bytes at that address (past image end if unmapped):");
        try {
            for (int i = 0; i < 4; i++) {
                w.println("    word " + i + ": " + f(neutralPtr + (long) i * 4)
                    + "  raw=" + hex(currentProgram.getMemory().getInt(toAddr(neutralPtr + (long) i * 4)) & 0xffffffffL));
            }
        } catch (Exception e) {
            w.println("  (unreadable: " + e.getMessage() + ")");
        }

        w.close();
        println("wrote " + out);
    }

    private long readPtr(long addr) throws Exception {
        return currentProgram.getMemory().getInt(toAddr(addr)) & 0xffffffffL;
    }

    private String f(long addr) {
        try {
            Address a = toAddr(addr);
            float v = Float.intBitsToFloat(currentProgram.getMemory().getInt(a));
            return String.valueOf(v);
        } catch (Exception e) {
            return "(unreadable: " + e.getMessage() + ")";
        }
    }

    private static String hex(long v) {
        return "0x" + Long.toHexString(v);
    }
}
