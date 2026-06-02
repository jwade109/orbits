import multiprocessing
import subprocess
import signal

def init_worker():
    signal.signal(signal.SIGINT, signal.SIG_IGN)

def work(cmd):
    subprocess.check_call(cmd, shell=False)

if __name__ == '__main__':

    subprocess.check_call("cargo build --release --bin headless_server", shell=False)
    subprocess.check_call("cargo build --release --bin main", shell=False)

    pool = multiprocessing.Pool(3, init_worker)
    try:
        pool.map(work, [
            "./target/release/headless_server.exe 5000 saves/ scenario_a",
            "./target/release/main.exe -a 127.0.0.1:5000 -s saves/scenario_a",
        ])
    except KeyboardInterrupt:
        print("Caught KeyboardInterrupt, terminating workers")
        pool.terminate()
        pool.join()
