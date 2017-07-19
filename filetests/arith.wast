(module
  (func (export "i32.add") (param $x i32) (param $y i32) (result i32)
    (i32.add (get_local $y) (get_local $x))
  )
)
