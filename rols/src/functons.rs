use rolscript as rs;
use rolscript::Error as RError;
use rolscript::*;

fn print(_this_value: &RValue, args: &[RValue]) -> Result<RValue, RError> {
    for arg in args {
        let s = rs::value_str(arg)?;
        print!("{}", s.as_str());
    }

    println!();
    Ok(rs::null().cast_value())
}

pub fn add_all() -> Result<(), RError> {
    let print = rs::RFunction::from_rust_func(print)?;
    rs::set_global_with_str("print", print.cast_value())?;

    Ok(())
}
