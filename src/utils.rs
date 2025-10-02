const DEFAULT_THREADS_NUM:usize = 1;
const CPU_2_THREAD_RATIO:usize = 2;

pub fn max_threads() -> usize {        
    let num_threads:usize = if let Ok(available_cpus) = std::thread::available_parallelism() {
        available_cpus.get()
    } else {
        DEFAULT_THREADS_NUM
    } * (CPU_2_THREAD_RATIO);

    num_threads
}