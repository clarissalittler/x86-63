# Registers, printing formats, and reversible stepping.
info registers
si
si
p $rbx
p/x $rcx
p/t $rbx
si
p $rcx
why
back
p $rcx
set $rbx = -5
p/d $rbx
p/x $ebx
back
c
info registers
