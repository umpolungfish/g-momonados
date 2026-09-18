// boot_snapshot: the kernel's boot self-imscription, on the card. Computes the
// static structural fields self_imscribe sets at boot (dynamic fields are zero
// at boot, so compute_tier collapses to tier=1 iff frob_order>0 or dialetheia).
//
// Device token ids (tok_id): 0 VINIT 1 TANCH 2 AFWD 3 AREV 4 CLINK 5 EVALT
// 6 FSPLIT 7 FFUSE 8 IMSCRIB 9 EVALF 10 ENGAGR 11 IFIX 12 FSPLIT3 13 FFUSE3
// 14 EVALI 15 ROTAT.
//
// out layout (unsigned int):
//   [0] frobenius_order  [1] period      [2..6] sig (L,F,D,X)
//   [6] token_diversity  [7] self_ref    [8] dialetheia_complete
//   [9] atomic_reentry   [10] bifurcation_revisited  [11] tier
__device__ void boot_snapshot(const unsigned char* P, unsigned int plen, unsigned int* out)
{
    unsigned int n = plen;

    // token diversity
    unsigned char seen[16]; for (int i=0;i<16;i++) seen[i]=0;
    for (unsigned int i=0;i<n;i++){ unsigned char t=P[i]; if (t<16) seen[t]=1; }
    unsigned int diversity=0; for (int i=0;i<16;i++) diversity+=seen[i];

    unsigned char self_ref = (n>0 && P[0]==P[n-1]) ? 1 : 0;

    // family signature: Logical, Frobenius, Dialetheia, Linear
    unsigned int L=0,F=0,D=0,X=0;
    for (unsigned int i=0;i<n;i++){
        unsigned char t=P[i];
        if (t==0||t==1||t==2||t==3||t==4||t==8||t==15) L++;
        else if (t==6||t==7||t==12||t==13) F++;
        else if (t==5||t==9||t==10||t==14) D++;
        else if (t==11) X++;
    }

    // frobenius order
    unsigned char fsplit=0,ffuse=0,fsplit3=0,ffuse3=0;
    int first_split=-1, first_fuse=-1;
    for (unsigned int i=0;i<n;i++){
        unsigned char t=P[i];
        if (t==6){ fsplit=1; if(first_split<0) first_split=(int)i; }
        else if (t==7){ ffuse=1; if(first_fuse<0) first_fuse=(int)i; }
        else if (t==12) fsplit3=1;
        else if (t==13) ffuse3=1;
    }
    unsigned int frob_order;
    if (fsplit3||ffuse3) frob_order=3;
    else if (!fsplit && !ffuse) frob_order=0;
    else if (fsplit && !ffuse) frob_order=1;
    else if (!fsplit && ffuse) frob_order=2;
    else frob_order = (first_split < first_fuse) ? 1 : 2;

    // dialetheia complete: presence + cyclic reachability of a gate from each ENGAGR
    unsigned char has_evalt=0, has_evalf=0, has_engagr=0;
    for (unsigned int i=0;i<n;i++){ unsigned char t=P[i]; if(t==5)has_evalt=1; else if(t==9)has_evalf=1; else if(t==10)has_engagr=1; }
    unsigned int dial=0;
    if (has_evalt && has_evalf && has_engagr) {
        dial=1;
        for (unsigned int i=0;i<n && dial;i++){
            if (P[i]==10){
                unsigned char found=0;
                for (unsigned int off=1; off<n; off++){ unsigned char t=P[(i+off)%n]; if(t==5||t==9){found=1;break;} }
                if(!found) dial=0;
            }
        }
    }

    // minimal period
    unsigned int period=n; if(n==0) period=1;
    for (unsigned int p=1;p<=n;p++){
        if (n%p==0){
            unsigned char ok=1;
            for (unsigned int i=p;i<n;i++){ if (P[i]!=P[i%p]){ ok=0; break; } }
            if (ok){ period=p; break; }
        }
    }

    // atomic re-entry: exactly one brancher and one merger
    unsigned int branchers=0, mergers=0;
    for (unsigned int i=0;i<n;i++){ unsigned char t=P[i]; if(t==6||t==12)branchers++; else if(t==7||t==13)mergers++; }
    unsigned int atomic_reentry = (branchers==1 && mergers==1) ? 1 : 0;
    unsigned int bifurcation = (atomic_reentry && self_ref) ? 1 : 0;

    // boot tier: dynamic fields are 0, so tier = 1 iff frob_order>0 or dialetheia
    unsigned int tier = (frob_order>0 || dial) ? 1 : 0;

    out[0]=frob_order; out[1]=period;
    out[2]=L; out[3]=F; out[4]=D; out[5]=X;
    out[6]=diversity; out[7]=self_ref; out[8]=dial;
    out[9]=atomic_reentry; out[10]=bifurcation; out[11]=tier;
}
