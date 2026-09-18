# Carried reciprocal correction

The descending Fermat correction now carries a fixed-point reciprocal at scale

\[
W=2^{2w+2}.
\]

Each root projection performs one seed division for `floor(W / 2B)`. Exact quotients thereafter use tape multiplication, structural shift by the scale exponent, and a one-step correction. When the Newton denominator decreases, the preceding reciprocal remains a lower approximation and is refined through

\[
R'=\left\lfloor\frac{R(2W-dR)}W\right\rfloor
\]

until `Rd <= W < (R+1)d`. Debug execution compares every reciprocal and quotient with tape `divmod`, then compares the final correction with the two-division binary reference from `8bbff11`.

The 511-bit control used one reciprocal seed division, six root refinements, thirty-four reciprocal refinements, and seven quotient multiply-shifts. Its forty-process median was 4.73 ms.

The 1023-bit control used four reciprocal seed divisions, twenty-nine root refinements, 177 reciprocal refinements, and thirty-three quotient multiply-shifts. Its median was 26.37 ms.

The carried reciprocal removes repeated quotient division exactly, but fixing the reciprocal to the exact integer after every denominator change costs more tape multiplications than the divisions it replaces. The next boundary is exact reciprocal fixation. Quotient-directed reciprocal precision can stop as soon as the quotient is fixed, without requiring the reciprocal itself to reach `floor(W/d)`.
