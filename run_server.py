import multiprocessing
import subprocess
import signal

def init_worker():
    signal.signal(signal.SIGINT, signal.SIG_IGN)

def work(cmd):
    subprocess.check_call(cmd, shell=False)

if __name__ == '__main__':

    subprocess.check_call("cargo build --release", shell=False)

    pool = multiprocessing.Pool(3, init_worker)
    try:
        pool.map(work, [
            "./target/release/server_app.exe",
            "./target/release/main.exe",
            "./target/release/main.exe",
        ])
    except KeyboardInterrupt:
        print("Caught KeyboardInterrupt, terminating workers")
        pool.terminate()
        pool.join()
