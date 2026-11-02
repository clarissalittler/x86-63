# Recursion walkthrough for Lecture 6 fact.s with the real I/O helpers.
break fact
c
input 5
c
bt
p $rdi
c
c
info frame
x/4gx $rsp
delete
c
p/x $rax
back
p $rax
set $rax = 7
p $rax
back
info breakpoints
