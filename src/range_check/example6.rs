// Goal: This is an extension of example5 that performs the range check using lookup table 
// It includes a further lookup table that contains a value num_bits. 
// For example it can be that our range is 8 bits, but we want to perform a range check on 4 bits.
// That's why we need this optimization.

use std::marker::PhantomData;

use halo2_proofs::{
    plonk::*,
    circuit::*,
    arithmetic::FieldExt, poly::Rotation
};

// create a submodule which is my table and use that
mod table;
use table::RangeCheckTable;

#[derive(Debug, Clone)]
/// A range-constrained value in the circuit produced by the RangeCheckConfig.
struct RangeConstrained<F: FieldExt, const RANGE: usize>(AssignedCell<Assigned<F>, F>);

#[derive(Debug, Clone)]

// WE ADD A FURTHER NUM_BITS COLUMN TO OUR CONFIG
struct RangeCheckConfig<F:FieldExt, const RANGE: usize, const LOOKUP_NUMBITS: usize, const LOOKUP_RANGE: usize> {
    value: Column<Advice>,
    num_bits: Column<Advice>,
    q_range_check: Selector,
    q_lookup: Selector,
    table: RangeCheckTable<F, LOOKUP_NUMBITS, LOOKUP_RANGE>
}

// Write the gate for our range check Config
// It's good practive to pass advice columns to the config (rather than creating it within the config)
// because these are very likely to be shared across multiple config
impl<F: FieldExt, const RANGE: usize, const LOOKUP_NUMBITS: usize, const LOOKUP_RANGE: usize> RangeCheckConfig<F, RANGE, LOOKUP_NUMBITS, LOOKUP_RANGE> {

    // REMEMBER THAT THE CONFIGURATION HAPPEN AT KEYGEN TIME
    fn configure(
        meta: &mut ConstraintSystem<F>,
        value: Column<Advice>,
        num_bits: Column<Advice> 
    ) -> Self {
        // Toggles the range check constraint
        let q_range_check = meta.selector();

        // Toggles the lookup argument
        // I use a complex selector for that. 
        // Simple selector cannot appear in lookup arguments.
        let q_lookup = meta.complex_selector();

        // We also need to configure our look up table and pass it to config
        let table =   RangeCheckTable::configure(meta);

        let config = Self {
            q_range_check,
            q_lookup,
            value,
            num_bits,
            table: table.clone()
        }; 

        // range-check gate
        // For a value v and a range R, check that v < R
        // v * (1 - v) * (2 - v) * ... (R - 1 - v) = 0 if v is any of these values! 
        meta.create_gate("range check", |meta| {
            let q_range_check = meta.query_selector(q_range_check);
            // note that we don't need to specify the rotation when querying the selctor
            // That's because the selector always get queried at the current row
            // While the advice columns get queried relatively to the selector offset, so we need to specify the relative rotation
            let value = meta.query_advice(value,Rotation::cur());

            // This is a closure that produce the expression defined previously 
            let range_check = |range: usize, value: Expression<F>| {
                (0..range).fold(value.clone(), |expr: Expression<F>, i: usize| {
                    expr * (Expression::Constant(F::from(i as u64)) - value.clone())
                })
            };
            // This is a way to return the constrain from our create_gate function. 
            // similar to what we were doing previously using "vec![s * (a + b - c)]"
            // this api, behind the scene, multiplies the specified constraint by the selector
            Constraints::with_selector(q_range_check, [("range check", range_check(RANGE, value))])
        });

        // range-check using lookup argument
        // Check that a value is contained within a lookup table of values 0..RANGE (exclusive)
        // IN THIS EXAMPLE we also want to lookup into the num_bits table
        // api to instantiate a lookup argument
        // Similar to create gate as an api so we need to query the selector and our value
        meta.lookup(|meta| {
            let q_lookup = meta.query_selector(q_lookup);
            // we add another advise column from the previous example
            let num_bits = meta.query_advice(num_bits, Rotation::cur());
            let value = meta.query_advice(value,Rotation::cur());

            // The meta.lookup api expect to return a vector of tuples, where the first element
            // is what you are looking at, and the second element is the corresponding table we are looking into
            // NOW WE ALSO LOOKUP num_bits from the instance column inside the num_bits lookup table
            vec![
                (q_lookup.clone() * value, table.value),
                (q_lookup * num_bits, table.num_bits)
            ]
        });

        config
    }

    // assign value to each cell inside the advise column
    // we can modify this assign function such that under a certain range enables the simple range check expression
    // and over a certain range enables the look up argument
    // the range passed in is the actual claimed range
    fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        value: Value<Assigned<F>>,
        num_bits: usize,
        range: usize
    ) -> Result<(), Error> {

        assert!(range <= LOOKUP_RANGE);

        if range < RANGE {
            layouter.assign_region(|| "Assign value", |mut region| {

                let offset = 0;
    
                // Enable q range check
                self.q_range_check.enable(&mut region, offset)?;

                // assign num bits
                region.assign_advice(
                    || "assign num_bits",
                    self.num_bits,
                    offset,
                    || Value::known(F::from(num_bits as u64))
                )?;

                // assign given value and return RangeConstrained struct
                region.assign_advice(
                    || "assign value",
                    self.value,
                    offset,
                    || value
                )?;

                Ok(())
            })
        } else {
            layouter.assign_region(|| "Assign value for lookup range check", |mut region| {

                let offset = 0;
    
                // Enable q range check
                self.q_lookup.enable(&mut region, offset)?;

                // assign given value and return RangeConstrained struct
                region.assign_advice(
                    || "assign value",
                    self.value,
                    offset,
                    || value
                )?;

                Ok(())
        })

}

}

}

// Now let's test it! Here we define a circuit with a single value. and in syntesize function we assign that value
#[cfg(test)]
mod tests {
    use core::num;

    use halo2_proofs::{
        circuit::floor_planner::V1,
        dev::{FailureLocation, MockProver, VerifyFailure},
        pasta::Fp,
        plonk::{Any, Circuit},
    };

    use super::*;

    #[derive(Default)]
    struct MyCircuit<F: FieldExt, const RANGE: usize, const LOOKUP_NUMBITS: usize, const LOOKUP_RANGE: usize> {
        value: Value<Assigned<F>>,
        large_value_num_bits: Option<usize>,
        large_value: Value<Assigned<F>>
    }

    impl<F: FieldExt, const RANGE: usize, const LOOKUP_NUMBITS: usize, const LOOKUP_RANGE: usize> Circuit<F> for MyCircuit<F, RANGE, LOOKUP_NUMBITS, LOOKUP_RANGE> {
        type Config = RangeCheckConfig<F, RANGE, LOOKUP_NUMBITS, LOOKUP_RANGE>;
        type FloorPlanner = V1;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
            let value = meta.advice_column();
            let num_bits = meta.advice_column();
            RangeCheckConfig::configure(meta, value, num_bits)
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<F>,
        ) -> Result<(), Error> {
            config.assign(layouter.namespace(|| "Assign value"), self.value, 0, RANGE)?;
            config.assign(layouter.namespace(|| "Assign value"), self.value, self.large_value_num_bits.unwrap(), LOOKUP_RANGE)?;
            // We need to load the values inside the lookup table! 
            config.table.load(&mut layouter)?;
            Ok(())
        }
    }

    #[test]
    fn test_range_check_3() {
        // our lookup table is 256 rows + last few rows or the advise colums 
        // are automatically allocated to random values which are bliding factors
        // so we need to use k=9
        let k = 9;
        const RANGE: usize = 8; // 3-bit value table
        const LOOKUP_NUMBITS: usize = 8; // 8-bit value table 
        const LOOKUP_RANGE: usize = 256; // 8-bit value table

        let circuit = MyCircuit::<Fp, RANGE, LOOKUP_NUMBITS, LOOKUP_RANGE> {
            value: Value::known(Fp::one().into()),
            large_value_num_bits: Some(4), // 8 which is 4 bits
            large_value: Value::known(Fp::from(8 as u64).into())
        };

        // // Successful cases large_value=0,1,2,3,4,5,6,7 (these should also pass the lookup range check)
        // for i in 0..RANGE {
        //     let circuit = MyCircuit::<Fp, RANGE, LOOKUP_NUMBITS, LOOKUP_RANGE> {
        //         value: Value::known(Fp::from(i as u64).into()),
        //         large_value : Value::known(Fp::from(i as u64).into())
        //     };

        //     let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        //     prover.assert_satisfied(); 
        // }

        // // Out-of-range `value = 8`
        // {
        //     let circuit = MyCircuit::<Fp, RANGE> {
        //         value: Value::known(Fp::from(RANGE as u64).into()),
        //     };
        //     let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        //     // prover.assert_satisfied(); // this should fail!
        //     assert_eq!(
        //         prover.verify(),
        //         Err(vec![VerifyFailure::ConstraintNotSatisfied {
        //             constraint: ((0, "range check").into(), 0, "range check").into(),
        //             location: FailureLocation::InRegion {
        //                 region: (0, "Assign value").into(),
        //                 offset: 0
        //             },
        //             cell_values: vec![(((Any::Advice, 0).into(), 0).into(), "0x8".to_string())]
        //         }])
        //     );
        // }
    }

    #[cfg(feature = "dev-graph")]
    #[test]
    fn print_range_check_2() {
        use plotters::prelude::*;

        let root = BitMapBackend::new("range-check-2-layout.png", (1024, 3096)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root
            .titled("Range Check 2 Layout", ("sans-serif", 60))
            .unwrap();

        let circuit = MyCircuit::<Fp, 8, 256> {
            value: Value::unknown(),
            large_value: Value::unknown()
        };
        halo2_proofs::dev::CircuitLayout::default()
            .render(3, &circuit, &root)
            .unwrap();
    }
} 