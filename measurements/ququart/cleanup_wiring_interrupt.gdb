set pagination off
set confirm off
set disable-randomization off
python
import os, signal, threading, time
def interrupt_debuggee():
    time.sleep(35)
    os.kill(os.getpid(), signal.SIGINT)
threading.Thread(target=interrupt_debuggee,daemon=True).start()
end
break _RNvMNtCsdpsOGJVkNCx_11g_momonados19ququart_folded_workNtB2_23QuquartFoldedWorkDevice23require_clean_workspace
commands
silent
printf "cleanup_boundary_entered\n"
continue
end
run
printf "debug_stop\n"
info registers rip rdi rsi
info proc mappings
bt 18
kill
quit
