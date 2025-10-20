use std::hint::spin_loop;
use statrs::distribution::{ContinuousCDF, Normal};

const DEFAULT_THREADS_NUM:usize = 1;
#[allow(dead_code)]
const CPU_2_THREAD_RATIO:usize = 2;

pub fn max_threads() -> usize {        
    let num_threads:usize = if let Ok(available_cpus) = std::thread::available_parallelism() {
        available_cpus.get() * CPU_2_THREAD_RATIO
    } else {
        DEFAULT_THREADS_NUM
    };

    num_threads
}

/// SpinWait struct is used wherever atomics are used to control resource access over a normal locking approach.
/// This usually comes into play whenever a compare_exchange or other function is being called.
/// As best practice, it is strongly recommended that the spin loop is terminated after a finite amount of iterations and 
/// an appropriate blocking syscall is made. This function achieves that.
/// SpinWait is designed to enable spin looping in a responsible manner 
pub struct SpinWait;

#[allow(dead_code)]
impl SpinWait {
    /// loop_while is an fn closure that acts like a predicate that is checked   
    /// ```
    /// use parallel_task::utils::SpinWait;
    /// use std::cell::Cell;
    /// struct Tester(Cell<usize>);
    /// let val = &Tester(Cell::new(0));
    /// let predicate = || { let y = val.0.get(); val.0.set(y + 1); val.0.get() < 10};
    /// SpinWait::loop_while(predicate);
    /// assert_eq!(val.0.get(),10);
    /// ``` 
    pub fn loop_while<F>(predicate:F)
    where F:Fn() -> bool  {       
        let mut spins = 0usize;
        while predicate() {
            Self::spin_backoff(&mut spins);
        }
        
    }

    /// loop_while_mut is an fnmut closure that acts like a predicate that is checked and may be modified
    /// like a compare_exchange or other similar case
    /// ```
    /// use parallel_task::utils::SpinWait;
    /// let mut val = 0;
    /// let predicate = || { val +=1; print!(" {}",val); val < 10 };
    /// SpinWait::loop_while_mut(predicate);
    /// assert_eq!(val,10);
    /// ```
    pub fn loop_while_mut<F>(mut predicate:F)
    where F:FnMut() -> bool  {
        let mut spins = 0usize;
        while predicate() {
            Self::spin_backoff(&mut spins);
        }
        
    }

    fn spin_backoff(spins: &mut usize) {
        static MAX_SPINS: usize = 128;                  
        if *spins < MAX_SPINS {
            spin_loop(); // tight cpu-friendly pause
            *spins += 1;                                           
        } else {      
            std::thread::yield_now();                               
            *spins /= 2; // regress back            
        }        
    }

}

fn mean_and_sample_std(data: &[f64]) -> Option<(f64, f64)> {
    let n = data.len();
    if n == 0 { return None; }
    let mean = data.iter().sum::<f64>() / n as f64;
    if n == 1 { return Some((mean, 0.0)); }
    let var = data.iter().map(|x| {
        let d = x - mean;
        d * d
    }).sum::<f64>() / (n as f64 - 1.0); // sample variance (n-1)
    Some((mean, var.sqrt()))
}

/// Return values outside the central `prob` interval (e.g. prob=0.90 for 90% central interval)
/// For a normal curve the two-sided z for central `prob` = inverse_cdf((1+prob)/2).
/// Here we precompute z for prob = 0.90: z ≈ 1.6448536269514722.
pub fn lower_leg_values_outside_central_interval(data: &[f64], prob: f64) -> Vec<(usize, f64)> {
    if data.is_empty() { return Vec::new(); }
    // only implemented for common probs; we hardcode z for 0.90 to avoid extra deps
    let z = normal_curve_z_value(prob);

    let (mean, std) = match mean_and_sample_std(data) {
        Some(ms) => ms,
        None => return Vec::new(),
    };

    let lower = mean - z * std;
    // let upper = mean + z * std;

    data.iter().cloned().enumerate().filter(|&(_,x)| x < lower).collect()
}

pub fn normal_curve_z_value(prob:f64) -> f64 {
    let normal = Normal::new(0.0, 1.0).unwrap();

    // For a central 90% interval, we want the upper tail at (1 + prob) / 2 = 0.95
    normal.inverse_cdf((1.0 + prob) / 2.0)    
}