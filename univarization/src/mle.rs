use core::ops::Index;

use crate::*;

pub struct EqPolynomial {
    x_vec: Vec<Scalar>,
}

impl EqPolynomial {

    pub fn new(x_vec: &Vec<Scalar>) -> Self {
        EqPolynomial {
            x_vec: x_vec.to_owned(),
        }
    }

    // compute all evals in O(n), from [Tha13]
    //  e.g.
    //      e0 = (1-x2)(1-x1)(1-x0)
    //      e1 = (1-x2)(1-x1) x0
    //      e2 = (1-x2) x1   (1-x0)
    //      e3 = (1-x2) x1    x0
    pub fn evals_over_hypercube(&self) -> Vec<Scalar> {
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

    // compute all evals in O(n*log(n))
    // NOTE: for testing, not used in production
    pub fn evals_over_hypercube_slow(&self) -> Vec<Scalar> {
        let x_vec = &self.x_vec;

        let log_size = self.x_vec.len();
        let full_size = pow_2(log_size);
        
        let mut evals = vec![Scalar::zero(); full_size];

        for i in 0..full_size {
            let mut prod_acc = Scalar::one();
            let i_bin = scalar_from_bits(i, log_size);
            for j in 0..log_size {
                let mut b = i_bin[j];

                let x = x_vec[j];

                let eq_j = (Scalar::one() - x) * (Scalar::one() - b) + (x * b);
                prod_acc *= eq_j;
            }
            evals[i] = prod_acc;
        }
        evals
    }
}

pub struct MLEPolynomial {
    pub num_var: usize,
    pub evals: Vec<Scalar>, // Hello, hypercube!
}

impl MLEPolynomial {
    pub fn new(vs: &[Scalar]) -> Self {
        let vs_len = vs.len();
        let mut evals = vs.to_vec();
        let full_len = vs_len.next_power_of_two();

        let num_var = log_2(full_len);

        let padded_len = full_len - vs_len;
        let padded_vec = vec![Scalar::zero(); padded_len];

        evals.extend(padded_vec);

        MLEPolynomial {
            num_var: num_var,
            evals: evals,
        }
    }

    pub fn len(&self) -> usize {
        self.evals.len()
    }

    // Folding the space from N-dim to (N-1)-dim
    pub fn fold_into_half(&mut self, rho: &Scalar) {
        let half = self.len() / 2;
        for i in 0..half {
            self.evals[i] = (Scalar::one() - rho) * self.evals[i] 
                + *rho * self.evals[i + half];
        }
        self.num_var -= 1;
    }

    pub fn evaluate(&self, rs: &[Scalar]) -> Scalar {
        assert_eq!(rs.len(), self.num_var);

        // chi is lagrange polynomials evaluated at rs
        let chi_vec = EqPolynomial::new(&rs.to_vec()).evals_over_hypercube();

        assert_eq!(chi_vec.len(), self.evals.len());
        (0..self.evals.len()).map(| i | chi_vec[i] * self.evals[i]).sum()
    }

}

impl Index<usize> for MLEPolynomial {
    type Output = Scalar;

    // TODO: inline
    fn index(&self, index: usize) -> &Self::Output {
        &(self.evals[index])
    }
}

mod tests {
    use super::*;

    #[test]
    fn test_eq_new() {
        let vs = Scalar::from_usize_vector(&[1,2,3]);
        let eq = EqPolynomial::new(&vs);
    }

    #[test]
    fn test_eq_evals_over_hypercube() {
        let vs = Scalar::from_usize_vector(&[1,2,3]);
        let eq = EqPolynomial::new(&vs);
        let evals = eq.evals_over_hypercube();
        let evals_prime = eq.evals_over_hypercube_slow();
        assert_eq!(evals, evals_prime);
        println!("evals={}", scalar_vector_to_string(&evals));
    }

    #[test]
    fn test_mle_new() {
        let vs = Scalar::from_usize_vector(&[1,2,3,4]);
        let mle = MLEPolynomial::new(&vs);
        assert_eq!(mle.len(), 4);
        assert_eq!(mle.num_var, 2);
        assert_eq!(mle.evals, Scalar::from_usize_vector(&[1,2,3,4]));
    }

    #[test]
    fn test_mle_new_again() {
        let vs = Scalar::from_usize_vector(&[1,2,3,4,5]);
        let mle = MLEPolynomial::new(&vs);
        assert_eq!(mle.len(), 8);
        assert_eq!(mle.num_var, 3);
        assert_eq!(mle.evals, Scalar::from_usize_vector(&[1,2,3,4,5,0,0,0]));
    }
}