use std::ops::{Deref, DerefMut};

use proc_macro2::{Punct, Spacing, TokenStream};
use quote::{ToTokens, TokenStreamExt};
use syn::token::Bracket;

pub struct Slice<T>(pub Vec<T>);

impl<T> FromIterator<T> for Slice<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(Vec::from_iter(iter))
    }
}

impl<T> ToTokens for Slice<T>
where
    T: ToTokens,
{
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut ts = TokenStream::new();
        Bracket::default().surround(&mut ts, |ts| {
            ts.append_separated(self.0.iter(), Punct::new(',', Spacing::Joint))
        });
        ts.to_tokens(tokens);
    }
}

impl<T> Deref for Slice<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Slice<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
