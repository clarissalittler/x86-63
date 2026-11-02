# Equates, scaled indexing, local branches, and memory examination.
p arrlen
p/x &arr
x/5dg &arr
break skip
c
p $rcx
p $rax
x/2i $pc
c
p $rcx
info breakpoints
break skip
c
p $rdi
