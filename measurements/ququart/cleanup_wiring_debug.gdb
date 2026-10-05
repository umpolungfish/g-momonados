set pagination off
set confirm off
set disable-randomization off
break _RNvMNtCsdpsOGJVkNCx_11g_momonados19ququart_folded_workNtB2_23QuquartFoldedWorkDevice23require_clean_workspace
run
printf "cleanup boundary reached\n"
info registers rip rdi rsi
bt
kill
quit
