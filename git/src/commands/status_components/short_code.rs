pub struct ShortCode{
    x: String,
    y: String
}

impl ShortCode{
  pub fn new(x: &str, y: &str)-> Self{
    ShortCode {x: x.to_string(), y: y.to_string()}
  }

  pub fn get_code(&mut self)-> String{
    format!("{}{}", self.x, self.y)
  }
}