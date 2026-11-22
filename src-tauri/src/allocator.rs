//! glibc's allocator, set to give memory back.
//!
//! By default glibc moves its mmap threshold up to the size of each large
//! block freed (as far as 32 MiB). After the first few decoded images (the
//! covers' backgrounds, ~8 MiB each) the next ones are no longer mapped on
//! their own but taken from the heaps of the threads that decode them, and
//! those heaps are seldom given back: 120 covers on 16 threads left 320 MB
//! that stayed after the threads were gone (and `malloc_trim` couldn't
//! free). A fixed threshold turns that off: every block over it is mapped on
//! its own and unmapped when freed (4 MB left in the same test).

/// Over this, a block is mapped on its own and given back when freed.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
const MMAP_THRESHOLD: i32 = 1024 * 1024;

/// Fixes glibc's mmap threshold. Call it first in `main`, before any thread.
pub fn give_memory_back() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        // M_MMAP_THRESHOLD in glibc's malloc.h.
        const M_MMAP_THRESHOLD: i32 = -3;
        unsafe extern "C" {
            fn mallopt(param: i32, value: i32) -> i32;
        }
        // SAFETY: mallopt only changes the allocator's settings; called before
        // any other thread starts.
        if unsafe { mallopt(M_MMAP_THRESHOLD, MMAP_THRESHOLD) } == 0 {
            log::warn!("could not set glibc's mmap threshold");
        }
    }
}

#[cfg(test)]
mod tests {
    /// Large blocks freed on other threads are given back: what was kept
    /// before the threshold was fixed. Measured in a process of its own
    /// (`measure_in_a_process_of_its_own`): the other tests, running at the
    /// same time, take memory too.
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    #[test]
    fn large_blocks_freed_on_threads_are_given_back() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "allocator::tests::measure_in_a_process_of_its_own", "--ignored", "--nocapture", "--test-threads=1"])
            .output()
            .unwrap();
        let printed = String::from_utf8_lossy(&output.stdout) + String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success() && printed.contains("1 passed"), "{printed}");
    }

    /// Run by `large_blocks_freed_on_threads_are_given_back`, alone.
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    #[test]
    #[ignore = "run alone by large_blocks_freed_on_threads_are_given_back"]
    fn measure_in_a_process_of_its_own() {
        fn anonymous_mb() -> u64 {
            let status = std::fs::read_to_string("/proc/self/status").unwrap();
            let line = status.lines().find(|l| l.starts_with("RssAnon:")).unwrap();
            line.split_whitespace().nth(1).unwrap().parse::<u64>().unwrap() / 1024
        }

        super::give_memory_back();
        let before = anonymous_mb();
        let threads: Vec<_> = (0..8)
            .map(|_| {
                std::thread::spawn(|| {
                    for _ in 0..16 {
                        // As a decoded background: 8 MiB written, then dropped.
                        let image = vec![1u8; 8 << 20];
                        std::hint::black_box(&image);
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        let after = anonymous_mb();
        assert!(after < before + 32, "{before} MB before, {after} MB after");
    }
}
