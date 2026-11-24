//! A biblioteca do provedor do Spotify: o login (o OAuth com os dois grants) e o que o teste avulso usa.
//!
//! O programa que o Player abre é o `main.rs`, que usa estes módulos. O `teste-audio` também, para provar o login e o
//! áudio sem abrir a interface.

pub mod oauth;
