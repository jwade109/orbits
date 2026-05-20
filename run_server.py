import multiprocessing
import subprocess
import signal

def init_worker():
    signal.signal(signal.SIGINT, signal.SIG_IGN)

def work(cmd):
    subprocess.check_call(cmd, shell=False)

if __name__ == '__main__':
    pool = multiprocessing.Pool(2, init_worker)
    try:
        pool.map(work, [
            "cargo run --release --bin server_app",
            "cargo run --release --bin test_app",
        ])
    except KeyboardInterrupt:
        print("Caught KeyboardInterrupt, terminating workers")
        pool.terminate()
        pool.join()
