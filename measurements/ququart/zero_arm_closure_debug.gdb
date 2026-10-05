set pagination off
set confirm off
set disable-randomization off
python
import os, signal, threading, time
def stop_at_limit():
    time.sleep(85)
    os.kill(os.getpid(), signal.SIGINT)
threading.Thread(target=stop_at_limit,daemon=True).start()
end
run
printf "terminal_or_cutoff_debug_stop\n"
info program
info registers rip
bt 32
kill
quit
