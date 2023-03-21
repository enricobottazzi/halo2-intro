## IsZero Gadget

This is a gadget that can be used inside other circuits. We just define the Chip here as we can reuse it across different circuit components. You can find it in the `is_zero.rs` file.

Now we'll see how a gadget can be used inside another circuit

## Example3 circuit

The circuit is executing this logic `f(a, b, c) = if a == b {c} else {a - b}`. We'll use the `IsZero` gadget to check if `a == b` which mean checking if `a - b` is zero or not.

In this example I'm using the constraint system set by the is_zero gadget + setting 2 new custom gate into our new circuit!

```
cargo test -- --nocapture test_example3
```

## Example4

Simple range check circuit `Config`. Given a value `v` and a maximum range that the value, we constraint the value. 

```
cargo test -- --nocapture test_range_check_1
```

```
cargo test --all-features -- --nocapture print_range_check_1
```

## Example5

Extend the range check using lookup arguments. Useful when you want to check it against a large range. Create the `Config` to support that

```
cargo test -- --nocapture test_range_check_2
```

```
cargo test --all-features -- --nocapture print_range_check_2
```

## Example6

Mix this range check `Config` with a word decompositon `Config`.


