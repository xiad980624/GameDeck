#[path="recheck_publishers.rs"] mod driver;
fn main(){std::env::set_var("GAMEDECK_RECHECK_PROVIDER_ONLY","1");driver::main();}
