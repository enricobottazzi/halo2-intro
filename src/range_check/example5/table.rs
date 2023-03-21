// here we are gonna define our lookup table
// A lookup table of values up to RANGE 
// e.g. RANGE = 256, values = [0..255]
// Once this is create it can be used inside our main config! 

use std::marker::PhantomData;

use halo2_proofs::{plonk::{Error, TableColumn, ConstraintSystem}, arithmetic::FieldExt, circuit::{Value, Layouter}, dev::metadata::Constraint};

// This is a table with a single column. 
// TableColumn is a Fixed Column
#[derive(Debug, Clone)]
pub(super) struct RangeCheckTable<F:FieldExt, const RANGE: usize> {
    pub(super) value: TableColumn,
    _marker: PhantomData<F>
}

impl<F:FieldExt, const RANGE: usize> RangeCheckTable<F, RANGE> {

    // create a configure function to allow to configure the table in the first place
    pub(super) fn configure(
        meta: &mut ConstraintSystem<F>
    ) -> Self {
        // API to create this special fixed colum
        let value = meta.lookup_table_column();
        Self {
            value,
            _marker: PhantomData
        }
    }


    // load function assign the values to our fixed table
    // This action is performed at key gen time
    pub(super) fn load(
         &self,
         layouter: &mut impl Layouter<F>
    ) -> Result<(), Error> {
        // firstly, for some RANGE we want to load all the values and assign it to the lookup table
        // assign_table is a special api that only works for lookup tables
        layouter.assign_table(|| "load range check table", |mut table| {
            let mut offset = 0;
            for i in 0..RANGE {
                table.assign_cell(|| "assign cell", self.value, offset, || Value::known(F::from(i as u64)))?;
                offset += 1;
            }


            Ok(())
        })
    }
}