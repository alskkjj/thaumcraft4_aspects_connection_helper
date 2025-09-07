use std::fmt::Display;

pub trait Evaluable {
    fn eval(&self, x: f64) -> Result<f64>;
}

pub struct NumberMapToValue {
    alpha: f64,
    beta: f64,
}

const ALPHA: f64 = 0.7f64;

#[derive(Debug, )]
pub enum MathError {
    Domain {
        valid_region: String,
        inputted: f64,
    },
    DivideByZero {
        formula: String,
    }
}

impl Display for MathError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MathError::Domain { valid_region, inputted } => {
                write!(f, "valid_region: {}, but input is {}", valid_region, inputted)
            },
            MathError::DivideByZero { formula } => {
                write!(f, "formula: {formula}")
            }
        }
    }
}

impl snafu::Error for MathError {
}

pub type Result<T> = std::result::Result<T, MathError>;

impl Default for NumberMapToValue {
    fn default() -> Self {
        Self::new(ALPHA).unwrap()
    }
}

impl NumberMapToValue {
    fn new(alpha: f64) -> Result<Self> {
        if alpha <= 0. || alpha >= 1.0 {
            return Err(
                MathError::Domain {
                    valid_region: "(0, 1)".to_string(),
                    inputted: alpha,
                }
            )
        }
        let a = 1000. * (1. - alpha);
        if f64::abs(a - 0.) < f64::EPSILON {
            return Err(MathError::DivideByZero { formula: "1000. * (1. - alpha)".to_string() })
        }
        let beta = alpha / a;
        Ok(Self {
            alpha,
            beta
        })
    }
}

impl Evaluable for NumberMapToValue {
    fn eval(&self, x: f64) -> Result<f64> {
        if 0. > x {
            return Err(MathError::Domain {
                valid_region: "[0, +inf)".to_string(),
                inputted: x,
            })
        }
        return Ok(if 0. <= x && x < 1000. {
            self.alpha * x / 1000.
        } else {
            let pa = -self.beta * ( x - 1000.);
            self.alpha + (1. - self.alpha) * (1. - pa.exp())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::NumberMapToValue;
    use super::Evaluable;

    #[test]
    fn test_map_to_value_function() {
        {
            let n = NumberMapToValue::default();
            let l = n.eval(f64::INFINITY).unwrap();
            assert!(f64::abs(l - 1.0) < f64::EPSILON);
        }
        {
            let n = NumberMapToValue::default();
            let l = n.eval(0.).unwrap();
            assert!(f64::abs(l - 0.) < f64::EPSILON);
        }
    }
}
