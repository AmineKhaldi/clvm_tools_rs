use std::collections::HashMap;
use std::rc::Rc;

use crate::compiler::sexp::{
    SExp
};

use crate::compiler::srcloc::{
    Srcloc
};

#[derive(Clone)]
pub struct CompileErr(pub Srcloc, pub String);

#[derive(Clone)]
pub struct CompiledCode(pub Srcloc, pub Rc<SExp>);

pub enum Callable {
    CallMacro(SExp),
    CallDefun(SExp),
    CallPrim(SExp),
    RunCompiler
}

pub fn list_to_cons(l: Srcloc, list: &Vec<Rc<SExp>>) -> SExp {
    if list.len() == 0 {
        return SExp::Nil(l.clone());
    }

    let mut result = SExp::Nil(l.clone());
    for i_reverse in 0..list.len() {
        let i = list.len() - i_reverse - 1;
        result = SExp::Cons(list[i].loc(), list[i].clone(), Rc::new(result));
    }

    return result;
}

#[derive(Clone)]
pub struct Binding {
    pub loc: Srcloc,
    pub name: Vec<u8>,
    pub body: Rc<BodyForm>
}

#[derive(Clone)]
pub enum BodyForm {
    Let(Srcloc, Vec<Rc<Binding>>, Rc<BodyForm>),
    Quoted(SExp),
    Value(SExp),
    Call(Srcloc, Vec<Rc<BodyForm>>)
}

#[derive(Clone)]
pub enum HelperForm {
    Defconstant(Srcloc, Vec<u8>, Rc<BodyForm>),
    Defmacro(Srcloc, Vec<u8>, Rc<SExp>, Rc<CompileForm>),
    Defun(Srcloc, Vec<u8>, bool, Rc<SExp>, Rc<BodyForm>)
}

#[derive(Clone)]
pub struct CompileForm {
    pub loc: Srcloc,
    pub args: Rc<SExp>,
    pub helpers: Vec<HelperForm>,
    pub exp: Rc<BodyForm>
}

#[derive(Clone)]
pub struct DefunCall {
    pub required_env: Rc<SExp>,
    pub code: Rc<SExp>
}

#[derive(Clone)]
pub struct PrimaryCodegen {
    pub prims: HashMap<Vec<u8>, Rc<SExp>>,
    pub constants: HashMap<Vec<u8>, Rc<SExp>>,
    pub macros: HashMap<Vec<u8>, Rc<SExp>>,
    pub defuns: HashMap<Vec<u8>, DefunCall>,
    pub parentfns: HashMap<Vec<u8>, Rc<SExp>>,
    pub env: Rc<SExp>,
    pub to_process: Vec<HelperForm>,
    pub final_expr: Rc<BodyForm>,
    pub final_code: Option<CompiledCode>
}

pub struct DefaultCompilerOpts {
    pub include_dirs: Vec<String>,
    pub filename: String,
    pub compiler: Option<PrimaryCodegen>,
    pub in_defun: bool,
    pub assemble: bool,
    pub stdenv: bool,
    pub start_env: Option<Rc<SExp>>
}

pub trait CompilerOpts {
    fn filename(&self) -> String;
    fn compiler(&self) -> Option<PrimaryCodegen>;
    fn in_defun(&self) -> bool;
    fn assemble(&self) -> bool;
    fn stdenv(&self) -> bool;
    fn start_env(&self) -> Option<Rc<SExp>>;

    fn set_assemble(&self, new_assemble: bool) -> Rc<dyn CompilerOpts>;
    fn set_in_defun(&self, new_in_defun: bool) -> Rc<dyn CompilerOpts>;
    fn set_stdenv(&self, new_stdenv: bool) -> Rc<dyn CompilerOpts>;
    fn set_compiler(&self, new_compiler: PrimaryCodegen) -> Rc<dyn CompilerOpts>;
    fn set_start_env(&self, start_env: Option<Rc<SExp>>) -> Rc<dyn CompilerOpts>;

    fn read_new_file(&self, inc_from: String, filename: String) -> Result<(String,String), CompileErr>;
    fn compile_program(&self, sexp: Rc<SExp>) -> Result<SExp, CompileErr>;
}

/* Frontend uses this to accumulate frontend forms */
pub struct ModAccum {
    pub loc: Srcloc,
    pub helpers: Vec<HelperForm>,
    pub exp_form: Option<CompileForm>
}

impl ModAccum {
    pub fn set_final(&self, c: &CompileForm) -> Self {
        ModAccum {
            loc: self.loc.clone(),
            helpers: self.helpers.clone(),
            exp_form: Some(c.clone())
        }
    }

    pub fn add_helper(&self, h: HelperForm) -> Self {
        let mut hs = self.helpers.clone();
        hs.push(h.clone());

        ModAccum {
            loc: self.loc.clone(),
            helpers: hs,
            exp_form: self.exp_form.clone()
        }
    }

    pub fn new(loc: Srcloc) -> ModAccum {
        ModAccum {
            loc: loc,
            helpers: Vec::new(),
            exp_form: None
        }
    }
}

impl CompileForm {
    pub fn loc(&self) -> Srcloc {
        return self.loc.clone();
    }

    pub fn to_sexp(&self) -> Rc<SExp> {
        let mut sexp_forms: Vec<Rc<SExp>> =
            self.helpers.iter().map(|x| x.to_sexp()).collect();
        sexp_forms.push(self.exp.to_sexp());

        Rc::new(SExp::Cons(
            self.loc.clone(),
            self.args.clone(),
            Rc::new(list_to_cons(self.loc.clone(), &sexp_forms))
        ))
    }
}

impl HelperForm {
    pub fn name(&self) -> Vec<u8> {
        match self {
            HelperForm::Defconstant(_,name,_) => name.clone(),
            HelperForm::Defmacro(_,name,_,_) => name.clone(),
            HelperForm::Defun(_,name,_,_,_) => name.clone()
        }
    }

    pub fn loc(&self) -> Srcloc {
        match self {
            HelperForm::Defconstant(l,_,_) => l.clone(),
            HelperForm::Defmacro(l,_,_,_) => l.clone(),
            HelperForm::Defun(l,_,_,_,_) => l.clone()
        }
    }

    pub fn to_sexp(&self) -> Rc<SExp> {
        match self {
            HelperForm::Defconstant(loc,name,body) => {
                Rc::new(list_to_cons(
                    loc.clone(),
                    &vec!(
                        Rc::new(SExp::atom_from_string(loc.clone(), &"defconstant".to_string())),
                        Rc::new(SExp::atom_from_vec(loc.clone(), &name)),
                        body.to_sexp(),
                    )
                ))
            },
            HelperForm::Defmacro(loc,name,args,body) => {
                Rc::new(list_to_cons(
                    loc.clone(),
                    &vec!(
                        Rc::new(SExp::atom_from_string(loc.clone(), &"defmacro".to_string())),
                        Rc::new(SExp::atom_from_vec(loc.clone(), &name)),
                        body.to_sexp()
                    )
                ))
            },
            HelperForm::Defun(loc,name,inline,arg,body) => {
                let di_string = "defun-inline".to_string();
                let d_string = "defun".to_string();
                Rc::new(list_to_cons(
                    loc.clone(),
                    &vec!(
                        Rc::new(SExp::atom_from_string(
                            loc.clone(),
                            if *inline {
                                &di_string
                            } else {
                                &d_string
                            }
                        )),
                        Rc::new(SExp::atom_from_vec(loc.clone(), &name)),
                        arg.clone(),
                        body.to_sexp()
                    )
                ))
            }
        }
    }
}

impl BodyForm {
    pub fn loc(&self) -> Srcloc {
        match self {
            BodyForm::Let(loc,_,_) => loc.clone(),
            BodyForm::Quoted(a) => a.loc(),
            BodyForm::Call(loc,_) => loc.clone(),
            BodyForm::Value(a) => a.loc()
        }
    }

    pub fn to_sexp(&self) -> Rc<SExp> {
        match self {
            BodyForm::Let(loc,bindings,body) => {
                let translated_bindings: Vec<Rc<SExp>> = bindings.iter().map(|x| x.to_sexp()).collect();
                let bindings_cons = list_to_cons(loc.clone(), &translated_bindings);
                let translated_body = body.to_sexp();
                Rc::new(SExp::Cons(
                    loc.clone(),
                    Rc::new(SExp::atom_from_string(loc.clone(), &"let".to_string())),
                    Rc::new(SExp::Cons(
                        loc.clone(),
                        Rc::new(bindings_cons),
                        Rc::new(SExp::Cons(
                            loc.clone(),
                            translated_body,
                            Rc::new(SExp::Nil(loc.clone()))
                        ))
                    ))
                ))
            },
            BodyForm::Quoted(body) => {
                Rc::new(SExp::Cons(
                    body.loc(),
                    Rc::new(SExp::atom_from_string(body.loc(), &"q".to_string())),
                    Rc::new(body.clone())
                ))
            },
            BodyForm::Value(body) => Rc::new(body.clone()),
            BodyForm::Call(loc,exprs) => {
                let converted: Vec<Rc<SExp>> = exprs.iter().map(|x| x.to_sexp()).collect();
                Rc::new(list_to_cons(loc.clone(), &converted))
            }
        }
    }
}

impl Binding {
    pub fn to_sexp(&self) -> Rc<SExp> {
        Rc::new(SExp::Cons(
            self.loc.clone(),
            Rc::new(SExp::atom_from_vec(self.loc.clone(), &self.name)),
            Rc::new(SExp::Cons(
                self.loc.clone(),
                self.body.to_sexp(),
                Rc::new(SExp::Nil(self.loc.clone()))
            ))
        ))
    }

    pub fn loc(&self) -> Srcloc {
        self.loc.clone()
    }
}

impl CompiledCode {
    pub fn loc(&self) -> Srcloc {
        return self.0.clone();
    }
}

impl PrimaryCodegen {
    pub fn add_constant(&self, name: &Vec<u8>, value: Rc<SExp>) -> Self {
        let mut codegen_copy = self.clone();
        codegen_copy.constants.insert(name.clone(), value);
        return codegen_copy;
    }

    pub fn add_macro(&self, name: &Vec<u8>, value: Rc<SExp>) -> Self {
        let mut codegen_copy = self.clone();
        codegen_copy.macros.insert(name.clone(), value);
        return codegen_copy;
    }

    pub fn add_defun(&self, name: &Vec<u8>, value: DefunCall) -> Self {
        let mut codegen_copy = self.clone();
        codegen_copy.defuns.insert(name.clone(), value);
        return codegen_copy;
    }

    pub fn set_env(&self, env: Rc<SExp>) -> Self {
        let mut codegen_copy = self.clone();
        codegen_copy.env = env.clone();
        return codegen_copy;
    }

}

pub fn with_heading(l: Srcloc, name: &String, body: Rc<SExp>) -> SExp {
    SExp::Cons(
        l.clone(),
        Rc::new(SExp::atom_from_string(l.clone(), &name.to_string())),
        body.clone()
    )
}

pub fn cons_of_string_map<X>(
    l: Srcloc,
    cvt_body: &dyn Fn(&X) -> Rc<SExp>,
    map: &HashMap<Vec<u8>, X>
) -> SExp {
    // Thanks: https://users.rust-lang.org/t/sort-hashmap-data-by-keys/37095/3
    let mut v: Vec<_> = map.into_iter().collect();
    v.sort_by(|x,y| x.0.cmp(&y.0));

    let sorted_converted: Vec<Rc<SExp>> = v.iter().map(|x| {
        Rc::new(SExp::Cons(
            l.clone(),
            Rc::new(SExp::QuotedString(l.clone(), '\"' as u8, x.0.to_vec())),
            Rc::new(SExp::Cons(
                l.clone(),
                cvt_body(x.1.clone()),
                Rc::new(SExp::Nil(l.clone()))
            ))
        ))
    }).collect();

    list_to_cons(l.clone(), &sorted_converted)
}

pub fn mapM<T,U,E>(f: &dyn Fn(&T) -> Result<U, E>, list: &Vec<T>) -> Result<Vec<U>, E> {
    let mut result = Vec::new();
    for e in list {
        let val = f(e)?;
        result.push(val);
    }
    return Ok(result);
}

pub fn decode_string(v: &Vec<u8>) -> String {
    return String::from_utf8_lossy(v).as_ref().to_string();
}

pub fn join_vecs_to_string(sep: Vec<u8>, vecs: &Vec<Vec<u8>>) -> String {
    let mut s = Vec::new();
    let mut comma = Vec::new();

    for elt in vecs {
        s.append(&mut comma.clone());
        s.append(&mut elt.to_vec());
        if comma.len() == 0 {
            comma = sep.clone();
        }
    }

    return decode_string(&s);
}
