# Fermat deficit projection

The carried correction now begins with

\[
E=2Bu+r
\]

and closes immediately when either `q = u` or `q = u - 1` satisfies the exact inequalities. All branch counters and division counts remain on IMASM tape.

The proposed feasible-deficit predecessor walk was measured before acceptance. Its bracket width was 77906446463541708325970550294429702289 for the 511-bit control and its single correction required 126 predecessor refinements. Across the four continued roots of the 1023-bit control, the total bracket width was 1140098261616285234063458583778830026901708187888030402289301710197877750917559 and refinement required 999 steps. Runtime rose to 18.16 ms at 1023 bits, so the bracket is not narrow in these products.

The retained morphism uses the quotient fast arms and constructs the exact feasible/infeasible bracket

\[
\left\lfloor\frac{E}{2B+u}\right\rfloor\le q<u.
\]

It bisects that bracket with structural tape shift-right-one. The predicate carries its feasible value incrementally as

\[
F(lo+h)=F(lo)+h(2B+2lo+h).
\]

The general path therefore performs exactly two tape divisions.

In the measured controls neither fast arm fired. The 511-bit case used one general correction, two divisions, and 128 binary refinements at a 3.79 ms median. The 1023-bit case used four general corrections, eight divisions, and 1016 binary refinements at a 17.47 ms median.

The division count is closed, while the measured bracket has the width of the carried correction itself. The next effective reduction must therefore collapse the monotone refinement breadth rather than narrow it with another scalar quotient.
