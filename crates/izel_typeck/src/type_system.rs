use crate::DefId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    /// Primitive types
    Prim(PrimType),
    
    /// User defined shapes/scrolls/duals
    Adt(DefId),
    
    /// Optional types (?T)
    Optional(Box<Type>),

    /// Cascade types (T!)
    Cascade(Box<Type>),
    
    /// Pointer types (*T or *~T)
    Pointer(Box<Type>, bool), // bool is mut
    
    /// Functions
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
    },
    
    /// Type variables (for inference)
    Var(usize),
    
    /// Generic parameters (<T>)
    Param(String),
    
    /// Tuple or anonymous shapes
    Static(Vec<(String, Type)>),
    
    /// Error sentinel
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimType {
    I8, I16, I32, I64, I128,
    U8, U16, U32, U64, U128,
    F32, F64,
    Bool,
    Str,
    Void,
    None,
}
