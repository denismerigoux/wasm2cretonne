(module
  (memory 1)
  (func $main (local i32)
    (i32.store (i32.const 0) (i32.const 7))
  )
  (start $main)
  (data (i32.const 0) "0000")
)
