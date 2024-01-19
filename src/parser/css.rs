#[derive(Debug)]
pub struct Style {
    pub rules: Vec<Rule>,
}

#[derive(Debug)]
pub struct Rule {
    pub selectors: Vec<Selector>,  
    pub declarations: Vec<Declaration>,
}

#[derive(Debug)]
pub enum Selector {

}

#[derive(Debug)]
pub enum Declaration {

}
