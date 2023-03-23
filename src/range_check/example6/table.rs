// here we are gonna define our lookup table
// A lookup table of values up to RANGE 
// e.g. RANGE = 256, values = [0..255]
// The table is tagged by an index k where k is the number of bits of the element in the `value` column.
// Once this is create it can be used inside our main config!  

use std::marker::PhantomData;

use halo2_proofs::{plonk::{Error, TableColumn, ConstraintSystem}, arithmetic::FieldExt, circuit::{Value, Layouter}, dev::metadata::Constraint};

// This is a table with a NOW 2 columns. 
// TableColumn is a Fixed Column
#[derive(Debug, Clone)]
pub(super) struct RangeCheckTable<F:FieldExt, const NUM_BITS: usize, const RANGE: usize> {
    pub(super) num_bits: TableColumn,
    pub(super) value: TableColumn,
    _marker: PhantomData<F>
}

impl<F:FieldExt, const NUM_BITS: usize, const RANGE: usize> RangeCheckTable<F, NUM_BITS, RANGE> {


    // create a configure function to allow to configure the table in the first place
    pub(super) fn configure(
        meta: &mut ConstraintSystem<F>
    ) -> Self {
        // check that 2^NUM_BITS = RANGE
        assert_eq!(1 << NUM_BITS, RANGE);

        // API to create these special fixed columns, which are look_up columns
        let value = meta.lookup_table_column();
        let num_bits = meta.lookup_table_column();
        Self {
            num_bits,
            value,
            _marker: PhantomData
        }
    }


    // load function assign the values to our fixed table
    // This action is performed at key gen time
    pub(super) fn load(&self, layouter: &mut impl Layouter<F>) -> Result<(), Error> {
        layouter.assign_table(
            || "load range-check table",
            |mut table| {
                let mut offset = 0;

                // Assign (num_bits = 1, value = 0)
                {
                    table.assign_cell(
                        || "assign num_bits",
                        self.num_bits,
                        offset,
                        || Value::known(F::one()),
                    )?;
                    table.assign_cell(
                        || "assign value",
                        self.value,
                        offset,
                        || Value::known(F::zero()),
                    )?;

                    offset += 1;
                }

                for num_bits in 1..=NUM_BITS {
                    for value in (1 << (num_bits - 1))..(1 << num_bits) {
                        table.assign_cell(
                            || "assign num_bits",
                            self.num_bits,
                            offset,
                            || Value::known(F::from(num_bits as u64)),
                        )?;
                        table.assign_cell(
                            || "assign value",
                            self.value,
                            offset,
                            || Value::known(F::from(value as u64)),
                        )?;
                        offset += 1;
                    }

                }

                Ok(())
            },
        )

    }
}