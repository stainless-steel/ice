use ffi;
use std::marker::PhantomData;
use std::mem;

use {Raw, Result};
use dimensions::Dimensions;
use stack_description::StackDescription;

/// A power grid.
pub struct PowerGrid<'l> {
    dimensions: Dimensions<'l>,
    raw: ffi::PowerGrid_t,
    phantom: PhantomData<&'l ffi::PowerGrid_t>,
}

impl<'l> PowerGrid<'l> {
    /// Map the power dissipation of the processing elements onto the thermal
    /// nodes.
    ///
    /// The size of `from` should be equal to the total number of elements in
    /// the floorplans of the source layers. The size of `onto` should be equal
    /// to the the total number of cells in the stack, which is `layers × rows ×
    /// columns`.
    pub fn distribute(&self, from: &[f64], onto: &mut [f64]) -> Result<()> {
        let layers = self.dimensions.layers();
        let cells = self.dimensions.rows() * self.dimensions.columns();
        if onto.len() != layers * cells {
            raise!("the size of the output buffer is invalid");
        }

        unsafe {
            let (mut i, mut j) = (0, 0);
            for k in 0..layers {
                match *self.raw.LayersProfile.offset(k as isize) {
                    ffi::TDICE_LAYER_SOURCE | ffi::TDICE_LAYER_SOURCE_CONNECTED_TO_AMBIENT => {
                        let floorplan = &**self.raw.FloorplansProfile.offset(k as isize);
                        let elements = floorplan.NElements as usize;

                        ffi::floorplan_matrix_multiply(
                            &floorplan.SurfaceCoefficients as *const _ as *mut _,
                            &mut onto[i..(i + cells)] as *const _ as *mut _,
                            &from[j..(j + elements)] as *const _ as *mut _);

                        j += elements;
                    },
                    _ => {},
                }
                i += cells;
            }
        }
        Ok(())
    }
}

impl<'l> Drop for PowerGrid<'l> {
    fn drop(&mut self) {
        unsafe { ffi::power_grid_destroy(&mut self.raw) };
    }
}

implement_raw!(PowerGrid, ffi::PowerGrid_t, l);

pub unsafe fn new<'l>(description: &'l StackDescription) -> Result<PowerGrid<'l>> {
    let mut raw = mem::uninitialized();
    ffi::power_grid_init(&mut raw);

    let dimensions = description.dimensions();

    let description = description.raw();
    let layers = ffi::get_number_of_layers(description.Dimensions);
    let cells = ffi::get_number_of_cells(description.Dimensions);

    success!(ffi::power_grid_build(&mut raw, layers, cells), "build the power grid");
    ffi::fill_power_grid(&mut raw, &description.StackElements as *const _ as *mut _);

    Ok(PowerGrid { dimensions: dimensions, raw: raw, phantom: PhantomData })
}
