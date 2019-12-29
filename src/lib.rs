//! Traits for taking ownership of values.

mod dereftake;
pub use self::dereftake::*;

mod take;
pub use self::take::Take;

mod intoowned;
pub use self::intoowned::IntoOwned;

#[cfg(test)]
mod tests {
}
