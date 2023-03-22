// Goal: I want to be able to check that the value witnessed in a given cell is within a given range.
// It looks like an advise colums where you witness a value and a selector that enables the range check constraint
//   value  | q_range_check
//   v      | 1
use std::marker::PhantomData;

use halo2_proofs::{
    plonk::*,
    circuit::*,
    arithmetic::FieldExt, poly::Rotation
};

#[derive(Debug, Clone)]
/// A range-constrained value in the circuit produced by the RangeCheckConfig.
struct RangeConstrained<F: FieldExt, const RANGE: usize>(AssignedCell<Assigned<F>, F>);

#[derive(Debug, Clone)]
// We also add a const range to the RANGE config such that we can specify the size of the range
// It's a good practice to use const generics to parameterize a type with a constant value
struct RangeCheckConfig<F:FieldExt, const RANGE: usize> {
    value: Column<Advice>,
    q_range_check: Selector,
    _marker: PhantomData<F>
}

// Write the gate for our range check Config
// It's good practive to pass advice columns to the config (rather than creating it within the config)
// because these are very likely to be shared across multiple config
impl<F: FieldExt, const RANGE: usize> RangeCheckConfig<F, RANGE> {

    fn configure(
        meta: &mut ConstraintSystem<F>,
        value: Column<Advice>
    ) -> Self {
        // Toggles the range check constraint
        let q_range_check = meta.selector();

        let config = Self {
            q_range_check,
            value,
            _marker: PhantomData
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

        config
    }

    // assign value to each cell inside the advise column
    fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        value: Value<Assigned<F>>
    ) -> Result<RangeConstrained<F, RANGE>, Error> {
        layouter.assign_region(|| "Assign value", |mut region| {

            let offset = 0;

            // Enable q range check
            self.q_range_check.enable(&mut region, offset)?;

            // assign given value and return RangeConstrained struct
            region.assign_advice(
                || "advice",
                self.value,
                offset,
                || value
            ).map(RangeConstrained)

    }) 
}

}

// Now let's test it! Here we define a circuit with a single value. and in syntesize function we assign that value
#[cfg(test)]
mod tests {
    use halo2_proofs::{
        circuit::floor_planner::V1,
        dev::{FailureLocation, MockProver, VerifyFailure},
        pasta::Fp,
        plonk::{Any, Circuit},
    };

    use super::*;

    #[derive(Default)]
    struct MyCircuit<F: FieldExt, const RANGE: usize> {
        value: Value<Assigned<F>>,
    }

    impl<F: FieldExt, const RANGE: usize> Circuit<F> for MyCircuit<F, RANGE> {
        type Config = RangeCheckConfig<F, RANGE>;
        type FloorPlanner = V1;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
            let value = meta.advice_column();
            RangeCheckConfig::configure(meta, value)
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<F>,
        ) -> Result<(), Error> {
            config.assign(layouter.namespace(|| "Assign value"), self.value)?;

            Ok(())
        }
    }

    #[test]
    fn test_range_check_1() {
        let k = 4;
        const RANGE: usize = 8; // 3-bit value

        // Successful cases i=0,1,2,3,4,5,6,7
        for i in 0..RANGE {
            let circuit = MyCircuit::<Fp, RANGE> {
                value: Value::known(Fp::from(i as u64).into()),
            };

            let prover = MockProver::run(k, &circuit, vec![]).unwrap();
            prover.assert_satisfied();
        }

        // Out-of-range `value = 8`
        {
            let circuit = MyCircuit::<Fp, RANGE> {
                value: Value::known(Fp::from(RANGE as u64).into()),
            };
            let prover = MockProver::run(k, &circuit, vec![]).unwrap();
            // prover.assert_satisfied(); // this should fail!
            prover.assert_satisfied();
            // assert_eq!(
            //     prover.verify(),
            //     Err(vec![VerifyFailure::ConstraintNotSatisfied {
            //         constraint: ((0, "range check").into(), 0, "range check").into(),
            //         location: FailureLocation::InRegion {
            //             region: (0, "Assign value").into(),
            //             offset: 0
            //         },
            //         cell_values: vec![(((Any::Advice, 0).into(), 0).into(), "0x8".to_string())]
            //     }])
            // );
        }
    }

    #[cfg(feature = "dev-graph")]
    #[test]
    fn print_range_check_1() {
        use plotters::prelude::*;

        let root = BitMapBackend::new("range-check-1-layout.png", (1024, 3096)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root
            .titled("Range Check 1 Layout", ("sans-serif", 60))
            .unwrap();

        let circuit = MyCircuit::<Fp, 8> {
            value: Value::unknown(),
        };
        halo2_proofs::dev::CircuitLayout::default()
            .render(3, &circuit, &root)
            .unwrap();
    }
}