set pagination off
set confirm off
set disable-randomization off
python
import os, signal, threading, time
def interrupt_debuggee():
    time.sleep(85)
    os.kill(os.getpid(), signal.SIGINT)
threading.Thread(target=interrupt_debuggee,daemon=True).start()
end
run
printf "terminal_or_cutoff_debug_stop\n"
info program
info registers rip
bt 24
kill
quit
