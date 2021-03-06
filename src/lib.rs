#![feature(use_extern_macros, wasm_custom_section, wasm_import_module)]
extern crate js_sys;
#[macro_use(array)]
extern crate ndarray;
extern crate parser;
extern crate wasm_bindgen;

use parser::parse;
use wasm_bindgen::prelude::*;
use ndarray::prelude::*;
use ndarray::SliceOrIndex;
use ndarray::Array;
use ndarray::{ArrayD, Dim, Ix, Ix2, IxDyn};

// need to be able to say the return type of  some of these functions is a certain dimension type
// ?? should I use an enum for thedim type?
#[wasm_bindgen]
#[derive(Clone)]
pub struct Nd {
    array: ArrayD<f32>,
}

//setting the consolelog for faster debugging
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s:&str);
}

//todo change the ndarr to nd style
#[wasm_bindgen]
impl Nd {
    pub fn from_arg(arr_arg: &str) -> Nd {
        //pub fn from_arg(arr_arg: &js::Array) -> String {
        // grr! the to_string turns it into a flat array...
        //let real_arr_arg = format!("[{}]", String::from(arr_arg.to_string())); // the surrounding [] are cut off in to_string()
        let real_arr_arg = arr_arg; // the surrounding [] are cut off in to_string()
        let (dimensions, numbers) = parse(&real_arr_arg);
        let ixdyn = IxDyn(&dimensions);
        let arr = Array::from_shape_vec(ixdyn, numbers).unwrap().into_dyn();
        Nd { array: arr }
    }
    pub fn from_ab(arr: &js_sys::Float32Array, dims: &js_sys::Array) -> Nd {
        let mut val_vec = vec![];
        let dim_vec = make_arr_usize(dims);
        arr.for_each(&mut |x, _, _| {
            val_vec.push(x);
        });
        let ixdyn = IxDyn(&dim_vec);
        Nd {
            array: Array::from_shape_vec(ixdyn, val_vec).unwrap(),
        }
    }

    #[wasm_bindgen(constructor)]
    pub fn make(arr_arg: &js_sys::Array) -> Nd {
        // simple test
        let str_arr = String::from(arr_arg.to_string()); //both coversions required due to the JsString used first
        let mut vec_str: Vec<_> = str_arr.split(",").collect();
        let filler = vec_str.remove(0).parse::<f32>().unwrap();
        let vec_dim: Vec<usize> = vec_str
            .into_iter()
            .map(|x| x.parse::<usize>().unwrap())
            .collect();
        let mut temp_arr = ArrayD::<f32>::zeros(IxDyn(&vec_dim));
        // todo explore whether the from_elem alt is faster than zero/fill method
        temp_arr.fill(filler);
        Nd { array: temp_arr }
    }
    // ?? ask nick if cloning is the only way to make this happen...
    // ?? how does testing work in this case?
    pub fn add(&self, other: &Nd) -> Nd {
        let _temp_self = self.array.clone();
        let _temp_other = other.array.clone();
        Nd {
            array: _temp_self + _temp_other,
        }
    }
    pub fn dot(&self, other: &Nd) -> Nd {
        let _temp_self = self.array.clone();
        let _temp_other = other.array.clone();
        let _temp_self_dottable = _temp_self.into_dimensionality::<Ix2>().unwrap();
        let _temp_other_dottable = _temp_other.into_dimensionality::<Ix2>().unwrap();
        Nd {
            array: _temp_self_dottable.dot(&_temp_other_dottable).into_dyn(),
        }
    }
    pub fn op(&self, operator: &str, other: &Nd) -> Nd {
        let _temp_self = self.array.clone();
        let _temp_other = other.array.clone();
        // broadcast default to other broadcasting
        let _broad_temp_other = _temp_other.broadcast(_temp_self.dim()).unwrap();
        Nd {
            array: match operator.trim() {
                "+" => _temp_self + _broad_temp_other,
                "-" => _temp_self - _broad_temp_other,
                "*" => _temp_self * _broad_temp_other,
                _ => panic!(),
            },
        }
    }
    pub fn get_slice_rust(&self, ind: JsValue) -> Self {
        //ind is a string which will contain the unpackable indexing structure
        //create a vector kind of thing from it, and rework following for iterating over the comma
        //separated entries
        let ind_string = ind.as_string().unwrap();
        let ind_vector = ind_string.split(',').collect::<Vec<&str>>();
        log(&format!("{:?}", ind_vector)[..]);
        let mut val_vec: Vec<SliceOrIndex> = vec![];
        for ind_str in ind_vector.iter() {
            // single integer index specified
            if let Ok(num) = ind_str.parse::<u32>() {
                val_vec.push(SliceOrIndex::Index(num as isize));
            } else {
                log(&format!("{} not counted for single ind", ind_str)[..]);
                // maybe extend this to the 2:5:1 syntax for indexing
                let pair = ind_str
                    .split(':')
                    .map(|e| e.parse::<u32>())
                    .collect::<Vec<Result<u32, std::num::ParseIntError>>>();
                //make into slice for destructuring matching
                match pair.as_slice() {
                    [Ok(num), _] => {
                        val_vec.push(SliceOrIndex::Slice {
                            start: *num as isize,
                            end: None,
                            step: 1_isize,
                        });
                    }
                    [Ok(num1), Ok(num2)] => {
                        val_vec.push(SliceOrIndex::Slice {
                            start: *num1 as isize,
                            end: Some(*num2 as isize),
                            step: 1_isize,
                        });
                    }
                    [_, Ok(num)] => {
                        val_vec.push(SliceOrIndex::Slice {
                            start: 0_isize,
                            end: Some(*num as isize),
                            step: 1_isize,
                        });
                    }
                    _ => panic!(),
                }
            }
        }
        log(&format!("{:?}", val_vec)[..]);
        // should have val_vec created by this point
        // !! slicing creates an array view, which might not be accepted for ND creation
        //      if so, look up how to create new ndarray from view
        let nd_slice_ob = ndarray::SliceInfo::<_, IxDyn>::new(val_vec).unwrap();
        Nd {
            array: self.array.slice(nd_slice_ob.as_ref()).to_owned(),
        }
    }
    pub fn get(&self, ind: &js_sys::Array) -> f32 {
        let rust_ind = make_arr_usize(ind);
        self.array[&rust_ind[..]]
    }
    pub fn set(&mut self, ind: &js_sys::Array, val: f32) {
        let rust_ind = make_arr_usize(ind);
        self.array[&rust_ind[..]] = val;
    }
    pub fn show(&self) -> String {
        format!("{:?}", self.array)
    }
}

fn make_arr_usize(arr: &js_sys::Array) -> Vec<usize> {
    let mut dim_vec = vec![];
    arr.for_each(&mut |x, _, _| dim_vec.push(x.as_f64().unwrap() as usize));
    dim_vec
}
