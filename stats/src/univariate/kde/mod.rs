//! Kernel density estimation

pub mod kernel;

use float::Float;
use rayon::prelude::*;

use univariate::Sample;

use self::kernel::Kernel;

/// Univariate kernel density estimator
pub struct Kde<'a, A, K>
where
    A: 'a + Float,
    K: Kernel<A>,
{
    bandwidth: A,
    kernel: K,
    sample: &'a Sample<A>,
}

impl<'a, A, K> Kde<'a, A, K>
where
    A: 'a + Float,
    K: Kernel<A>,
{
    /// Creates a new kernel density estimator from the `sample`, using a kernel and estimating
    /// the bandwidth using the method `bw`
    pub fn new(sample: &'a Sample<A>, kernel: K, bw: Bandwidth<A>) -> Kde<'a, A, K> {
        Kde {
            bandwidth: bw.estimate(sample),
            kernel,
            sample,
        }
    }

    /// Returns the bandwidth used by the estimator
    pub fn bandwidth(&self) -> A {
        self.bandwidth
    }

    /// Maps the KDE over `xs`
    ///
    /// - Multihreaded
    pub fn map(&self, xs: &[A]) -> Box<[A]> {
        xs.par_iter()
            .map(|&x| self.estimate(x))
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    /// Estimates the probability density of `x`
    pub fn estimate(&self, x: A) -> A {
        let _0 = A::cast(0);
        let slice = self.sample;
        let h = self.bandwidth;
        let n = A::cast(slice.len());

        let sum = slice
            .iter()
            .fold(_0, |acc, &x_i| acc + self.kernel.evaluate((x - x_i) / h));

        sum / h / n
    }
}

/// Method to estimate the bandwidth
pub enum Bandwidth<A>
where
    A: Float,
{
    /// Use this value as the bandwidth
    Manual(A),
    /// Use Silverman's rule of thumb to estimate the bandwidth from the sample
    Silverman,
}

impl<A> Bandwidth<A>
where
    A: Float,
{
    fn estimate(self, sample: &Sample<A>) -> A {
        match self {
            Bandwidth::Silverman => {
                let factor = A::cast(4. / 3.);
                let exponent = A::cast(1. / 5.);
                let n = A::cast(sample.len());
                let sigma = sample.std_dev(None);

                sigma * (factor / n).powf(exponent)
            }
            Bandwidth::Manual(bw) => bw,
        }
    }
}

#[cfg(test)]
macro_rules! test {
    ($ty:ident) => {
        mod $ty {
            use quickcheck::TestResult;

            use univariate::kde::kernel::Gaussian;
            use univariate::kde::{Bandwidth, Kde};
            use univariate::Sample;

            // The [-inf inf] integral of the estimated PDF should be one
            quickcheck! {
                fn integral(size: usize, start: usize) -> TestResult {
                    const DX: $ty = 1e-3;

                    if let Some(v) = ::test::vec::<$ty>(size, start) {
                        let slice = &v[start..];
                        let data = Sample::new(slice);
                        let kde = Kde::new(data, Gaussian, Bandwidth::Silverman);
                        let h = kde.bandwidth();
                        // NB Obviously a [-inf inf] integral is not feasible, but this range works
                        // quite well
                        let (a, b) = (data.min() - 5. * h, data.max() + 5. * h);

                        let mut acc = 0.;
                        let mut x = a;
                        let mut y = kde.estimate(a);

                        while x < b {
                            acc += DX * y / 2.;

                            x += DX;
                            y = kde.estimate(x);

                            acc += DX * y / 2.;
                        }

                        TestResult::from_bool(relative_eq!(acc, 1., epsilon = 2e-5))
                    } else {
                        TestResult::discard()
                    }
                }
            }
        }
    };
}

#[cfg(test)]
mod test {
    test!(f32);
    test!(f64);
}
