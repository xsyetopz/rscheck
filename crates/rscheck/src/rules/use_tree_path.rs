pub(super) fn flatten(tree: &syn::UseTree) -> Option<syn::Path> {
    match tree {
        syn::UseTree::Path(path) => {
            let mut segments = syn::punctuated::Punctuated::new();
            segments.push(path.ident.clone().into());
            let mut tail = flatten(&path.tree)?;
            segments.extend(tail.segments);
            tail.segments = segments;
            Some(tail)
        }
        syn::UseTree::Name(name) => Some(syn::Path::from(name.ident.clone())),
        _ => None,
    }
}
