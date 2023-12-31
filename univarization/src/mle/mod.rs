use core::ops::Index;
use core::fmt::Display;
use core::fmt;
use std::ops::IndexMut;
use core::cmp::min;
use log::debug;
use std::collections::HashMap;

use crate::*;
use crate::bits::*;

pub mod evals;
pub mod coeffs_sparse;
pub mod evals_sparse;

pub struct EqPolynomial {
    x_vec: Vec<Scalar>,
}

impl EqPolynomial {

    /// Returns a new eq polynomial with the given `x_vec`.
    /// 
    /// The formal definition of *eq polynomial* is:
    /// 
    /// ```
    ///   eq(X[], Y[]) = \prod_{0 <= i < N}((1 - X_i) * (1 - Y_i) + X_i * Y_i)
    /// ```
    /// 
    /// In practice, however, the first argument is a vector of random (Field) 
    /// elements, generated by the verifier, and the second argument is always a vector
    /// of a bits of a scalar, i.e. 
    /// 
    /// ```
    /// Y[] = [y_0, y_1, ..., y_{n-1}],
    /// ```
    /// 
    /// where y_i = 0 or 1. Thus, we can simplify the definition as:
    /// 
    /// ```
    ///   eq_y(X[]) = \prod_{i}((1 - X_i) * (1 - y_i) + X_i * y_i)
    ///```
    /// 
    /// where 
    /// 
    /// ```
    ///   y = y_0 + y_1 * 2 + y_2 * 4 + ... + y_{n-1} * 2^{n-1},
    /// ```
    /// or, we say `bits_LE(y) = [y_0, y_1, ..., y_{n-1}]`
    /// 
    /// e.g.  if `n = 3`, and `y = 0b011`, then
    /// 
    /// ```
    ///   Y[3] = [1, 1, 0]
    /// ```
    /// 
    /// and 
    /// 
    /// ```
    ///   eq_y(X_0,X_1,X_2) = X0 * X1 * (1 - X2)
    /// ```
    /// 
    /// # Arguments
    /// 
    /// - x_vec: the vector of random (Field) elements from the verifier
    ///  
    pub fn new(x_vec: &Vec<Scalar>) -> Self {
        EqPolynomial {
            x_vec: x_vec.to_owned(),
        }
    }

    
    /// Compute eq(r_vec, i_rec) in O(log N). 
    /// 
    /// Or, it returns a specific evaluation on some vertex of the hypercube.
    /// 
    /// Remember that eq polynomial is a grand product of `(1-X_i)` or `X_i`
    /// according to the bits of `i_rec`, the length of which is `n = log(N)`.
    /// 
    /// ```
    ///   eq_i(X[]) = \prod_{i}((1 - X_i) * (1 - B_i) + X_i * B_i)
    /// ```
    /// where `B_i=bits_LE(i)` is the i-th bit of `i_rec`.
    /// 
    /// 
    pub fn eval(&self, i: usize) -> Scalar {
        let x_log_vec = &self.x_vec;
        let i_bits = bits_LE(i, x_log_vec.len());

        // EQ = \prod_{i}((1 - x_i) * (1 - r_i) + x_i * r_i)
        (0..x_log_vec.len()).map(|i| 
            if i_bits[i] {x_log_vec[i]} else {Scalar::from(1) - x_log_vec[i]}
            ).product()
    }

    /// Compute eq(r_vec, x_vec) in O(log N). 
    /// 
    /// ```
    ///   eq(X[], Y[]) = \prod_{i}((1 - X_i) * (1 - Y_i) + X_i * Y_i)
    /// ```
    /// 
    /// 
    pub fn evaluate(&self, r_vec: &[Scalar]) -> Scalar {
        let x_vec = &self.x_vec;
        assert_eq!(x_vec.len(), r_vec.len());

        // EQ = \prod_{i}((1 - x_i) * (1 - r_i) + x_i * r_i)
        x_vec.iter().zip(r_vec.iter()).map(|(&a, &b)| 
            (Scalar::one() - a) * (Scalar::one() - b) + a * b
            ).product()
    }

    /// Compute all evaluations over the hypercube in O(N), from [Tha13].
    ///  
    ///
    ///             ^ X1
    ///             |
    ///             |
    ///             |e2          e3
    ///             +-----------+
    ///            /           /|
    ///        e6 / |      e7 / |
    ///          +-----------+  |
    ///          |  |        |  |
    ///          |  + - - - -|  + -----------------> X0
    ///          | / e0      | / e1
    ///          |           |/
    ///          +-----------+
    ///         /e4          e5
    ///        /
    ///       ∟ X2
    /// 
    /// If `n = 2^3 = 8`, and `x_vec = [r0, r1, r2]`, then
    /// ```
    ///      e0 = (1-r0)(1-r1)(1-r2)
    ///      e1 =  r0   (1-r1)(1-r2)
    ///      e2 = (1-r0) r1   (1-r2)
    ///      e3 =  r0    r1   (1-r2)
    ///      e0 = (1-r0)(1-r1) r2
    ///      e1 =  r0   (1-r1) r2
    ///      e2 = (1-r0) r1    r2
    ///      e3 =  r0    r1    r2
    /// ```
    /// Returns a vector of size `2^n`, where the i-th element is `e{i}`
    ///
    ///
    ///     // Compute all evaluations over the hypercube in O(n), from [Tha13]
    // The computation is similar to the reverse of folding.
    // NOTE: Particularly, the cost is only `n` field multiplications.
    //
    // x_vec = [X0, X1, ..., Xn]
    // e.g.
    //      e0 = (1-x0)(1-x1)(1-x2)
    //      e1 = x0    (1-x1)(1-x2) 
    //      e2 = (1-x0) x1   (1-x2)   
    //      e3 = x1     x0   (1-x2) 
    pub fn evals_over_hypercube(&self) -> Vec<Scalar> {
        let x_vec = &self.x_vec;

        let log_size = self.x_vec.len();
        let full_size = pow_2(log_size);
        
        let mut evals = vec![Scalar::one(); full_size];
        let mut half = 1;
        for i in 0..log_size {
            for j in 0..half {
                evals[j+half] = evals[j] * x_vec[i];

                // Normally, we should have computed `evals[j]` via 
                //    evals[j] = evals[j] * (Scalar::one() - x_vec[i])
                // However we can save one multiplication by computing
                //    evals[j] = evals[j] * (Scalar::one() - x_vec[i])
                //             = evals[j] - evals[j] * x_vec[i]
                //             = evals[j] - evals[j+half]
                // evals[j] = evals[j] * (Scalar::one() - x_vec[i]);
                evals[j] = evals[j] - evals[j+half];
            }
            half *= 2;
        }
        evals
    }
    
    /// TODO: obsoleted, remove it
    pub fn evals_over_hypercube_rev(&self) -> Vec<Scalar> {
        let x_vec = &self.x_vec;

        let log_size = self.x_vec.len();
        let full_size = pow_2(log_size);
        
        let mut evals = vec![Scalar::one(); full_size];
        let mut s = 1;
        for i in 0..log_size {
            s *= 2;
            for j in (0..s).rev().step_by(2) {
                let v = evals[j/2];
                evals[j] = v * x_vec[i];
                evals[j-1] = v * (Scalar::one() - x_vec[i]);
            }
        }
        evals
    }

    /// Compute all evaluations over the hypercube in O(n*log(n))
    /// NOTE: only for testing, not used in production
    pub fn evals_over_hypercube_slow(&self) -> Vec<Scalar> {
        let x_vec = &self.x_vec;

        let log_size = self.x_vec.len();
        let full_size = pow_2(log_size);
        
        let mut evals = vec![Scalar::zero(); full_size];

        for i in 0..full_size {
            let mut prod_acc = Scalar::one();
            let i_bin = scalar_from_bits_LE(i, log_size);
            for j in 0..log_size {
                let b = i_bin[j];

                let x = x_vec[j];

                let eq_j = (Scalar::one() - x) * (Scalar::one() - b) + (x * b);
                prod_acc *= eq_j;
            }
            evals[i] = prod_acc;
        }
        evals
    }

    // TODO: 
    pub fn to_evals() -> Vec<Scalar> {
        unimplemented!();
    }
}

/// Interpolate the evaluations into coefficients over hypercube.
/// The asymptotic complexity is O(N * log^2(N)).
/// 
/// TODO: can we compute in place (without memory allocation)?
///
/// The argument evals: the evaluations of the MLE over hypercube
/// 
/// ```
///    evals = [0b000: e0 paired with (1-X0)(1-X1)(1-X2),
///             0b001: e1 paired with  X0   (1-X1)(1-X2),
///             0b010: e2 paired with (1-X0)  X1  (1-X2),
///             0b011: e3 paired with  X0     X1  (1-X2),
///             0b100: e4 paired with (1-X0)(1-X1)  X2  ,
///             0b101: e5 paired with  X0   (1-X1)  X2  ,
///             0b110: e6 paired with (1-X0)  X1    X2  , 
///             0b111: e7 paired with  X0     X1    X2  ,
///            ]    
/// ```
/// Return coeffs: the coefficients of the MLE
/// 
/// ```
///   coeffs = [0b000: c0 of constant term, 
///             0b001: c1 of X0           ,
///             0b010: c2 of    X1        ,
///             0b011: c3 of X0 X1        ,
///             0b100: c4 of       X2     ,
///             0b101: c5 of X0    X2     ,
///             0b110: c6 of    X1 X2     ,
///             0b111: c7 of X0 X1 X2     ,
///            ]
/// ```

pub fn compute_coeffs_from_evals(evals: &Vec<Scalar>) -> Vec<Scalar> {
    let mut coeffs = evals.clone();
    let len = coeffs.len();
    assert!(len.is_power_of_two());
    let num_var = log_2(len);

    let mut half = len / 2;
    for _i in 0..num_var {
        let b = len / half;
        for j in (0..b).step_by(2) {
            for k in 0..half {
                let a = coeffs[j*half + k];
                coeffs[(j+1)*half + k] -= a;
            }
        }
        half = half / 2;
    };
    coeffs
}

/// Compute all evaluations over hypercube from coefficients.
/// The asymptotic complexity is O(N*log^2(N)).
/// 
/// Arugment coeffs: the coefficients of the polynomial (non-sparse form)
/// 
/// ```
///   coeffs = [0b000: c0 of constant term, 
///             0b001: c1 of X0           ,
///             0b010: c2 of    X1        ,
///             0b011: c3 of X0 X1        ,
///             0b100: c4 of       X2     ,
///             0b101: c5 of X0    X2     ,
///             0b110: c6 of    X1 X2     ,
///             0b111: c7 of X0 X1 X2     ,
///            ]
/// ```
/// 
/// Return evals: the evaluations of the polynomial (non-sparse form)
/// 
/// ```
///    evals = [0b000: e0 paired with (1-X0)(1-X1)(1-X2),
///             0b001: e1 paired with  X0   (1-X1)(1-X2),
///             0b010: e2 paired with (1-X0)  X1  (1-X2),
///             0b011: e3 paired with  X0     X1  (1-X2),
///             0b100: e4 paired with (1-X0)(1-X1)  X2  ,
///             0b101: e5 paired with  X0   (1-X1)  X2  ,
///             0b110: e6 paired with (1-X0)  X1    X2  , 
///             0b111: e7 paired with  X0     X1    X2  ,
///            ]    
/// ```
///
pub fn compute_evals_from_coeffs(num_var: usize, coeffs: &[Scalar]) -> Vec<Scalar> {
    let len = pow_2(num_var);
    assert!(coeffs.len() <= len);
    let mut evals = coeffs.to_vec();

    // Padding zeros to match the length of the hypercube
    let zeros = vec![Scalar::zero(); len - coeffs.len()];
    evals.extend(zeros.into_iter());

    // Initialize the position of folding
    let mut half = len / 2; // number of blocks

    for _i in 0..num_var {
        for j in 0..half {
            let s = len/half; // size of each block
            for k in 0..s/2 {  // tranverse over the top-half of the block 
                let a = evals[j*s + k];
                evals[j*s + k + (s/2)] += a;
            }
        }
        half = half / 2;
    }
    evals
}