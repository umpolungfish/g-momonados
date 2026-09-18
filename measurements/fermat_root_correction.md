# Tape-native Fermat root correction

The admitted-destination root state now carries

\[
t=D-b^2,\qquad c=a-b.
\]

For a scheduled jump `J`, the correction morphism forms

\[
B=b+J,\qquad E=t+2Jc
\]

and solves for the largest tape integer `q` satisfying

\[
q(2B+q)\le E.
\]

It then updates

\[
b'=B+q,\quad c'=c-q,\quad t'=E-q(2B+q),\quad g'=2b'+1-t'.
\]

The phase leaf reads exact closure directly as `t' == 0`. It does not square the carried root. Debug execution retains `isqrt_from(D')` and `D'-b'^2` only as independent assertions against the correction result.

The strict Newton seed is

\[
q<\left\lfloor\frac{E}{2B}\right\rfloor+1,
\]

which costs one tape division. Computing the additional concavity bound did not reduce the measured Newton iterations and added another division, so it is absent from execution.

The 511-bit control used two root projections and five correction iterations at a forty-process median of 3.07 ms. The 1023-bit control used five root projections and twenty-five correction iterations at a median of 5.79 ms. These match the preceding runtime rung while removing full-radicand square root and terminal root squaring from admitted continuation.
