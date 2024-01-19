#[derive(Debug)]
pub struct Style {
    pub rules: Vec<Rule>,
}

#[derive(Debug)]
struct Rule {
    pub selectors: Vec<Selector>,  
    pub declarations: Vec<Declaration>,
}

#[derive(Debug)]
enum Selector {

}

#[derive(Debug)]
enum Declaration {

}
