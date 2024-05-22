// This file is an exact copy from the Acala project (https://github.com/AcalaNetwork/Acala/),
// which is licensed under the GNU General Public License.
//
// For more info of its license, please check the Acala project

use proc_macro::TokenStream;
use proc_macro2::Literal;
use quote::quote;
use syn::{parse_macro_input, Expr, ExprLit, Ident, ItemEnum, Lit, LitByteStr, LitStr};

#[proc_macro_attribute]
pub fn generate_function_selector(_: TokenStream, input: TokenStream) -> TokenStream {
	let item = parse_macro_input!(input as ItemEnum);

	let ItemEnum {
		attrs,
		vis,
		enum_token,
		ident,
		variants,
		..
	} = item;

	let mut ident_expressions: Vec<Ident> = vec![];
	let mut variant_expressions: Vec<Expr> = vec![];
	for variant in variants {
		if let Some((_, Expr::Lit(ExprLit { lit, .. }))) = variant.discriminant {
			if let Lit::Str(token) = lit {
				let selector = module_evm_utility::get_function_selector(&token.value());
				// println!("method: {:?}, selector: {:?}", token.value(), selector);
				ident_expressions.push(variant.ident);
				variant_expressions.push(Expr::Lit(ExprLit {
					lit: Lit::Verbatim(Literal::u32_suffixed(selector)),
					attrs: Default::default(),
				}));
			} else {
				panic!("Not method string: `{:?}`", lit);
			}
		} else {
			panic!("Not enum: `{:?}`", variant);
		}
	}

	(quote! {
		#(#attrs)*
		#vis #enum_token #ident {
			#(
				#ident_expressions = #variant_expressions,
			)*
		}
	})
	.into()
}

#[proc_macro]
pub fn keccak256(input: TokenStream) -> TokenStream {
	let lit_str = parse_macro_input!(input as LitStr);

	let result = module_evm_utility::sha3_256(&lit_str.value());

	let eval = Lit::ByteStr(LitByteStr::new(result.as_ref(), proc_macro2::Span::call_site()));

	quote!(#eval).into()
}
