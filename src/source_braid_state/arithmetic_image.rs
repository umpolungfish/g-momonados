//! Arithmetic-valued source edges. A FOUR cell selects a radix-four affine
//! transformation of [x²,x,1], rather than branching over its output rails.
//! The equality observer remains an explicit two-address relation; its trace
//! is not the ordinary norm of this three-coordinate arithmetic vector.
use crate::{belnap::B4,word_tape::WordTape};
use serde_json::{json,Value};

#[derive(Clone,Debug)]
pub struct ArithmeticEdge { coefficients:[WordTape;9] }

impl ArithmeticEdge {
    pub fn for_coordinate(value:B4)->Self {
        let (truth,falsehood)=value.to_wh2();
        let digit=WordTape::from_small(2*u64::from(truth)+u64::from(falsehood));
        Self{coefficients:[WordTape::from_small(16),digit.shl(3),digit.mul(&digit),
            WordTape::zero(),WordTape::from_small(4),digit,
            WordTape::zero(),WordTape::zero(),WordTape::one()]}
    }

    pub fn apply(&self, input:&[WordTape;3])->[WordTape;3] {
        core::array::from_fn(|row| (0..3).fold(WordTape::zero(),|sum,column|
            sum.add(&self.coefficients[3*row+column].mul(&input[column]))))
    }

    pub fn coefficients(&self)->&[WordTape;9] {&self.coefficients}
}

/// A correlated functional image of every address below the exact source.
/// `source_root` names the retained source DAG in its original shared arena.
/// All cells share four arithmetic edges. The complete source bound remains
/// attached; independent marginals are never substituted for the family.
#[derive(Clone,Debug)]
pub struct SquareImage {
    pub source:WordTape,
    pub source_root:usize,
    pub cells:usize,
    edges:[ArithmeticEdge;4],
}

#[derive(Clone,Debug)]
pub struct ReturnedAddress {
    pub address:WordTape,
    pub integer:WordTape,
    pub modular:WordTape,
    pub quotient:WordTape,
}

/// Numerical readout of one addressed member of the retained axis family.
/// A caller must still obtain that member through the executing source family;
/// this operation is not a source-family selector or a factor producer.
#[derive(Clone,Debug)]
pub struct AxisReadout {
    pub first:WordTape,
    pub second:WordTape,
    pub index:WordTape,
    pub left:ReturnedAddress,
    pub right:ReturnedAddress,
    pub divisor:WordTape,
    pub factors:Option<(WordTape,WordTape)>,
}

impl SquareImage {
    /// Project the entire functional zero-product family away from both
    /// coordinate axes. This changes its support without enumerating it.
    /// Obtaining an address from this implicit support is a separate readout.
    pub fn project_proper_axis_family(&self)->ProperAxisFamily {
        ProperAxisFamily{image:self.clone()}
    }
    pub fn new(source:WordTape,source_root:usize)->Result<Self,String> {
        if source<=WordTape::one(){return Err("arithmetic image requires N > 1".into());}
        let cells=source.bit_len().div_ceil(2);
        Ok(Self{source,source_root,cells,
            edges:[B4::N,B4::T,B4::F,B4::B].map(ArithmeticEdge::for_coordinate)})
    }

    pub fn return_at(&self,address:&WordTape)->Result<ReturnedAddress,String> {
        if address>=&self.source{return Err("address outside arithmetic source family".into());}
        let mut register=[WordTape::zero(),WordTape::zero(),WordTape::one()];
        for cell in (0..self.cells).rev() {
            let value=B4::from_wh2(address.bit_at(2*cell+1),address.bit_at(2*cell));
            let index=[B4::N,B4::T,B4::F,B4::B].iter().position(|&v|v==value).unwrap();
            register=self.edges[index].apply(&register);
        }
        let [integer,recovered,one]=register;
        if recovered!=*address || !one.is_one(){return Err("arithmetic edge lost its source address".into());}
        let (quotient,modular)=integer.divmod(&self.source).ok_or("zero arithmetic source")?;
        Ok(ReturnedAddress{address:recovered,integer,modular,quotient})
    }

    pub fn inverse_at(&self,returned:&ReturnedAddress)->Result<WordTape,String> {
        let image=self.return_at(&returned.address)?;
        returned.modular.add(&self.source).sub(&image.modular)
            .ok_or("negative arithmetic inverse")?.divmod(&self.source)
            .map(|(_,r)|r).ok_or_else(||"zero arithmetic source".into())
    }

    pub fn collision_at(&self,left:&WordTape,right:&WordTape)->Result<bool,String> {
        Ok(self.return_at(left)?.modular==self.return_at(right)?.modular)
    }

    /// Push the paired equality observation through the reversible
    /// difference/sum chart. No square-image output rails are constructed.
    pub fn equality_chart(&self,left:&WordTape,right:&WordTape)
        ->Result<(WordTape,WordTape),String> {
        if left>=&self.source || right>=&self.source || !self.source.bit_at(0) {
            return Err("equality chart requires retained addresses and odd N".into());
        }
        let a=left.add(&self.source).sub(right).ok_or("negative difference chart")?
            .divmod(&self.source).ok_or("zero chart source")?.1;
        let b=left.add(right).divmod(&self.source).ok_or("zero chart source")?.1;
        Ok((a,b))
    }

    pub fn inverse_chart(&self,a:&WordTape,b:&WordTape)
        ->Result<(WordTape,WordTape),String> {
        if a>=&self.source || b>=&self.source || !self.source.bit_at(0) {
            return Err("inverse chart requires source residues and odd N".into());
        }
        let half=self.source.add(&WordTape::one()).shr(1);
        let x=half.mul(&a.add(b)).divmod(&self.source).ok_or("zero chart source")?.1;
        let difference=b.add(&self.source).sub(a).ok_or("negative inverse chart")?;
        let y=half.mul(&difference).divmod(&self.source).ok_or("zero chart source")?.1;
        Ok((x,y))
    }

    /// Contract the entire second zero-product coordinate analytically.
    /// The resulting family is b=t*(N/gcd(a,N)), 0<=t<gcd(a,N).
    /// This does not evaluate the remaining sum over the first coordinate.
    pub fn contract_zero_product_fibre(&self,a:&WordTape)->Result<ZeroProductFibre,String> {
        if a>=&self.source{return Err("fibre address outside source".into());}
        let count=a.gcd(&self.source);
        let (step,remainder)=self.source.divmod(&count).ok_or("zero fibre count")?;
        if !remainder.is_zero(){return Err("fibre gcd did not divide source".into());}
        Ok(ZeroProductFibre{source:self.source.clone(),first:a.clone(),step,count})
    }

    /// Carry an axis-fibre member back through the reversible chart and
    /// extract gcd(|x-y|,N) from those actual retained source addresses.
    pub fn paired_axis_readout(&self,first:&WordTape,index:&WordTape)
        ->Result<AxisReadout,String> {
        let fibre=self.contract_zero_product_fibre(first)?;
        let second=fibre.member(index)?;
        let (x,y)=self.inverse_chart(first,&second)?;
        let left=self.return_at(&x)?;
        let right=self.return_at(&y)?;
        if left.modular!=right.modular{return Err("axis member lost its square-return equality".into());}
        let (a,b)=self.equality_chart(&x,&y)?;
        if a!=*first || b!=second{return Err("axis chart did not recover both carried coordinates".into());}
        let difference=if x>=y{x.sub(&y)}else{y.sub(&x)}.ok_or("negative axis difference")?;
        let divisor=difference.gcd(&self.source);
        if divisor!=fibre.count{return Err("axis gcd changed under the retained chart".into());}
        let factors=if divisor>WordTape::one() && divisor<self.source {
            let (quotient,remainder)=self.source.divmod(&divisor).ok_or("zero axis divisor")?;
            if !remainder.is_zero() || divisor.mul(&quotient)!=self.source {
                return Err("proper axis divisor did not close the exact source product".into());
            }
            Some((divisor.clone(),quotient))
        }else{None};
        Ok(AxisReadout{first:first.clone(),second,index:index.clone(),left,right,divisor,factors})
    }

    pub fn record(&self)->Value {
        let edges=[B4::N,B4::T,B4::F,B4::B].iter().zip(&self.edges).map(|(value,edge)|
            json!({"value":value.name(),"coefficients":edge.coefficients().iter()
                .map(WordTape::as_word).collect::<Vec<_>>()})).collect::<Vec<_>>();
        json!({"schema":"source-square-arithmetic-image/v1","source_word":self.source.as_word(),
            "source_root":self.source_root,"belnap_cells":self.cells,
            "shared_edge_count":4,"arithmetic_coordinate_count":3,
            "coordinate_semantics":["integer square return","retained source address","one"],
            "edges":edges,
            "source_bound":"0 <= address < N",
            "collision_observer":"[return_at(x).modular = return_at(y).modular] on two retained addresses",
            "collision_trace_evaluated":false,
            "representation":"arithmetic functional image; not amplitude coordinates or a materialized joint DAG"})
    }
}

/// Whole-family support projection. This is not a numerical address selector.
#[derive(Clone,Debug)]
pub struct ProperAxisFamily { image:SquareImage }

impl ProperAxisFamily {
    pub fn amplitude_at(&self,a:&WordTape,b:&WordTape)->Result<WordTape,String> {
        if a.is_zero() || b.is_zero() || a>=&self.image.source || b>=&self.image.source {
            return Ok(WordTape::zero());
        }
        let remainder=a.mul(b).divmod(&self.image.source).ok_or("zero projection source")?.1;
        Ok(if remainder.is_zero(){WordTape::one()}else{WordTape::zero()})
    }

    /// Analytically contract the projected second-coordinate family.
    /// Removing b=0 changes its extent from gcd(a,N) to gcd(a,N)-1.
    pub fn fibre_extent_at(&self,a:&WordTape)->Result<WordTape,String> {
        if a.is_zero() || a>=&self.image.source{return Ok(WordTape::zero());}
        a.gcd(&self.image.source).sub(&WordTape::one()).ok_or_else(||"negative projected fibre extent".into())
    }

    pub fn read_member(&self,a:&WordTape,index:&WordTape)->Result<AxisReadout,String> {
        let extent=self.fibre_extent_at(a)?;
        if index>=&extent{return Err("no addressed member in projected proper family".into());}
        let returned=self.image.paired_axis_readout(a,&index.add(&WordTape::one()))?;
        if !self.amplitude_at(&returned.first,&returned.second)?.is_one() || returned.factors.is_none() {
            return Err("projected zero-product member failed proper factor closure".into());
        }
        Ok(returned)
    }

    pub fn record(&self)->Value {
        json!({"schema":"proper-axis-family-projection/v1","source_word":self.image.source.as_word(),
            "source_root":self.image.source_root,
            "support":"0<a<N; 0<b<N; a*b=0 mod N",
            "fibre_extent":"a=0:0; 0<a<N:gcd(a,N)-1",
            "member":"b=(index+1)*N/gcd(a,N), index<fibre_extent",
            "axes_removed":true,"representation":"implicit arithmetic family",
            "address_selected":false,"normalization_computed":false})
    }
}

#[derive(Clone,Debug)]
pub struct ZeroProductFibre {
    pub source:WordTape,
    pub first:WordTape,
    pub step:WordTape,
    pub count:WordTape,
}

impl ZeroProductFibre {
    pub fn member(&self,index:&WordTape)->Result<WordTape,String> {
        if index>=&self.count{return Err("fibre index outside retained family".into());}
        let second=self.step.mul(index);
        if second>=self.source || !self.first.mul(&second).divmod(&self.source)
            .ok_or("zero fibre source")?.1.is_zero() {
            return Err("zero-product fibre member did not close".into());
        }
        Ok(second)
    }
}
