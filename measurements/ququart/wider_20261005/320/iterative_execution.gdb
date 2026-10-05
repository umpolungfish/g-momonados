set pagination off
set confirm off
set disable-randomization off
set $ququart_stages = 0
break <g_momonados::ququart_folded_work::QuquartFoldedWorkDevice as g_momonados::ququart_factor::QuquartPhaseDevice>::controlled_multiply
commands
silent
set $ququart_stages = $ququart_stages + 1
printf "ququart_controlled_stage_entered=%d\n", $ququart_stages
continue
end
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
python
if gdb.selected_inferior().pid:
    gdb.execute("info registers rip")
    gdb.execute("info proc mappings")
    gdb.execute("bt")
    gdb.execute("kill")
end
quit
