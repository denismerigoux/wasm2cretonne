(module
  (func (export "i32.no_fold_shl_shr_s") (param $x i32) (result i32)
    (i32.shr_s (i32.shl (get_local $x) (i32.const 1)) (i32.const 1)))
)
