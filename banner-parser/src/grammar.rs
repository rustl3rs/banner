extern crate pest;

#[derive(Parser)]
#[grammar = "grammar/banner.pest"]
pub struct BannerParser;
