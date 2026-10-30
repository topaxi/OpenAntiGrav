;; NaN battery: writes the raw bits of NaNs made several ways to memory at 0.
;; Each result is the kind of value whose bits differ between CPUs (x86 makes a
;; negative default NaN, AArch64 a positive one; payloads propagate or not).
(module
  (memory (export "memory") 1)
  (func (export "nans") (param $a f32) (param $b f32) (param $p f32) (param $da f64)
    ;; 0: 0/0, the default NaN the CPU makes
    (f32.store (i32.const 0) (f32.div (local.get $a) (local.get $a)))
    ;; 4: sqrt(-1)
    (f32.store (i32.const 4) (f32.sqrt (f32.neg (local.get $b))))
    ;; 8: inf - inf
    (f32.store (i32.const 8) (f32.sub (f32.div (local.get $b) (local.get $a)) (f32.div (local.get $b) (local.get $a))))
    ;; 12: a payload NaN (0x7fa00001, passed in) plus 1: does the payload survive?
    (f32.store (i32.const 12) (f32.add (local.get $p) (local.get $b)))
    ;; 16: min(payload NaN, 1)
    (f32.store (i32.const 16) (f32.min (local.get $p) (local.get $b)))
    ;; 20: max(-0, +0) and 24: min(+0, -0): signed zeros, no NaN
    (f32.store (i32.const 20) (f32.max (f32.neg (local.get $a)) (local.get $a)))
    (f32.store (i32.const 24) (f32.min (local.get $a) (f32.neg (local.get $a))))
    ;; 32: f64 0/0
    (f64.store (i32.const 32) (f64.div (local.get $da) (local.get $da)))
    ;; 40: a payload NaN through neg (a bit operation, never canonicalised by the spec)
    (f32.store (i32.const 40) (f32.neg (local.get $p)))
    ;; 44: payload NaN demoted through f64 and back
    (f32.store (i32.const 44) (f32.demote_f64 (f64.promote_f32 (local.get $p))))
  )
)
