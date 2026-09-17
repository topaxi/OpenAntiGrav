/* Read-only probe for the HD weapon-pad handover thread's Next Steps 2-3
 * (docs/ghidra/functions/ps3-hdfury-eu/pads.md carries the results): diff
 * WeaponPad_Importer's and SpeedupPad_Importer's vtables at slots 0, 3, 5,
 * and find every function that references the
 * literal address 0x00aec2c0 (WeaponPad_Importer's `this+0x1b0` .bss target)
 * through its own TOC slot, per toolchain.md's "before concluding a fixed
 * address has no consumer, search the whole image for that address as a
 * 4-byte literal" rule.
 *
 * Read-only: no renames, no comments, no database writes. Run against the
 * already-imported /ps3-hdfury-eu/EBOOT.elf with -process, not -import.
 *
 * @category OpenAntiGrav
 */

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.address.AddressSet;
import ghidra.program.model.address.AddressSetView;
import ghidra.program.model.lang.Register;
import ghidra.program.model.lang.RegisterValue;
import ghidra.program.model.listing.CodeUnit;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionIterator;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.listing.Listing;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.mem.MemoryAccessException;
import ghidra.program.model.mem.MemoryBlock;
import ghidra.program.model.scalar.Scalar;

public class PadImporterVtableDiff extends GhidraScript {

    private static final long WEAPON_VTABLE = 0x0086a730L;
    private static final long SPEEDUP_VTABLE = 0x0086a780L;
    private static final int SLOTS = 17;
    private static final long TARGET_GLOBAL = 0x00aec2c0L;

    @Override
    public void run() throws Exception {
        String out = getScriptArgs().length > 0 ? getScriptArgs()[0]
            : "/tmp/pad-importer-vtable-diff.txt";
        PrintWriter w = new PrintWriter(new FileWriter(out));

        w.println("# WeaponPad_Importer (" + hex(WEAPON_VTABLE)
            + ") vs SpeedupPad_Importer (" + hex(SPEEDUP_VTABLE) + ") vtables, " + SLOTS + " slots");
        long[] weapon = readVtable(WEAPON_VTABLE, w, "weapon");
        long[] speedup = readVtable(SPEEDUP_VTABLE, w, "speedup");

        w.println();
        w.println("# Diff (slot: weapon vs speedup, marked DIFFERS where the target differs)");
        for (int i = 0; i < SLOTS; i++) {
            boolean differs = weapon[i] != speedup[i];
            w.println("slot " + i + ": weapon=" + describe(weapon[i]) + " speedup=" + describe(speedup[i])
                + (differs ? "  DIFFERS" : ""));
        }

        w.println();
        w.println("# Whole-image scan for the literal " + hex(TARGET_GLOBAL) + " (WeaponPad_Importer's this+0x1b0 target)");
        findLiteralConsumers(TARGET_GLOBAL, w);

        w.close();
        println("wrote " + out);
    }

    private long[] readVtable(long addr, PrintWriter w, String label) throws MemoryAccessException {
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
        if (fn == null) {
            fn = currentProgram.getFunctionManager().getFunctionContaining(a);
        }
        if (fn != null) {
            return hex(target) + " (" + fn.getName() + ")";
        }
        return hex(target);
    }

    /* Search every initialized memory block for the target address as a
     * big-endian 4-byte word (the loader is 32-bit addressing under
     * A2ALT-32addr). For every hit, treat it as a candidate TOC slot and, for
     * every function whose own r2 (assigned by AssignPs3R2FromOpd) makes that
     * slot reachable within a signed 16-bit displacement, check whether the
     * function's disassembly actually contains a d(r2) instruction at that
     * displacement - the two-TOC and sign-extension traps toolchain.md names. */
    private void findLiteralConsumers(long target, PrintWriter w) throws Exception {
        Memory mem = currentProgram.getMemory();
        Register r2reg = currentProgram.getRegister("r2");
        if (r2reg == null) {
            w.println("no r2 register on this language - skipping");
            return;
        }

        java.util.List<Address> hits = new java.util.ArrayList<>();
        for (MemoryBlock block : mem.getBlocks()) {
            if (!block.isInitialized() || !block.isLoaded()) {
                continue;
            }
            Address start = block.getStart();
            Address end = block.getEnd();
            long len = end.subtract(start) + 1;
            for (long off = 0; off + 4 <= len; off += 4) {
                Address a = start.add(off);
                try {
                    long v = mem.getInt(a) & 0xffffffffL;
                    if (v == target) {
                        hits.add(a);
                    }
                } catch (MemoryAccessException e) {
                    // unmapped sub-range inside an otherwise-initialized block
                }
            }
        }
        w.println("literal hits (candidate TOC slots holding " + hex(target) + "): " + hits.size());
        for (Address h : hits) {
            w.println("  " + h);
        }

        int checked = 0;
        int found = 0;
        FunctionIterator fns = currentProgram.getFunctionManager().getFunctions(true);
        while (fns.hasNext()) {
            Function fn = fns.next();
            RegisterValue rv = null;
            try {
                rv = currentProgram.getProgramContext().getRegisterValue(r2reg, fn.getEntryPoint());
            } catch (Exception e) {
                continue;
            }
            if (rv == null || rv.getUnsignedValue() == null) {
                continue;
            }
            long r2 = rv.getUnsignedValue().longValue();
            checked++;
            for (Address h : hits) {
                long disp = h.getOffset() - r2;
                if (disp < -0x8000L || disp > 0x7fffL) {
                    continue;
                }
                if (functionUsesDisplacement(fn, disp)) {
                    found++;
                    w.println("  candidate consumer: " + fn.getName() + " @ " + fn.getEntryPoint()
                        + " r2=" + hex(r2) + " slot=" + h + " disp=" + disp);
                }
            }
        }
        w.println("functions checked (had an r2 assigned): " + checked);
        w.println("candidate consumer hits: " + found);
    }

    private boolean functionUsesDisplacement(Function fn, long disp) {
        Listing listing = currentProgram.getListing();
        AddressSetView body = fn.getBody();
        InstructionIterator ii = listing.getInstructions(body, true);
        while (ii.hasNext()) {
            Instruction insn = ii.next();
            int n = insn.getNumOperands();
            for (int op = 0; op < n; op++) {
                Object[] reprs = insn.getOpObjects(op);
                boolean usesR2 = false;
                Long scalarVal = null;
                for (Object o : reprs) {
                    if (o instanceof Register && "r2".equals(((Register) o).getName())) {
                        usesR2 = true;
                    }
                    if (o instanceof Scalar) {
                        scalarVal = ((Scalar) o).getSignedValue();
                    }
                }
                if (usesR2 && scalarVal != null && scalarVal.longValue() == disp) {
                    return true;
                }
            }
        }
        return false;
    }

    private static String hex(long v) {
        return "0x" + Long.toHexString(v);
    }
}
