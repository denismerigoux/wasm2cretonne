(module
  (memory 1)
  (func $main (local i32)
      (set_local 0 (i32.sub (i32.const 5) (i32.const 4)))
  )
  (start $main)
  (data (i32.const 0) "abcdefgh")
)
