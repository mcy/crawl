//! Operator overloads.

use std::mem;
use std::mem::MaybeUninit;
use std::ops::Add;
use std::ops::AddAssign;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ops::Div;
use std::ops::DivAssign;
use std::ops::Mul;
use std::ops::MulAssign;
use std::ops::Neg;
use std::ops::Sub;
use std::ops::SubAssign;

use num::Zero;

use crate::geo::Point;
use crate::geo::Rect;

const N: usize = 2;

#[inline]
fn make<T>(mut f: impl FnMut() -> T) -> [T; 2] {
  [f(), f()]
}

#[inline]
fn map<T: Sized, U>(x: [T; 2], mut f: impl FnMut(T) -> U) -> [U; 2] {
  unsafe {
    let x0 = mem::transmute_copy::<_, [MaybeUninit<T>; 2]>(&x);
    let mut y = mem::transmute_copy::<_, [MaybeUninit<U>; 2]>(&MaybeUninit::<
      [U; 2],
    >::uninit());

    mem::forget(x);

    for i in 0..N {
      y[i].as_mut_ptr().write(f(x0[i].as_ptr().read()))
    }
    mem::transmute_copy(&y)
  }
}

#[inline]
fn zip<T, U, V>(x: [T; 2], y: [U; 2], mut f: impl FnMut(T, U) -> V) -> [V; 2] {
  unsafe {
    let x0 = mem::transmute_copy::<_, [MaybeUninit<T>; 2]>(&x);
    let y0 = mem::transmute_copy::<_, [MaybeUninit<U>; 2]>(&y);
    let mut z = mem::transmute_copy::<_, [MaybeUninit<V>; 2]>(&MaybeUninit::<
      [V; 2],
    >::uninit());

    mem::forget(x);
    mem::forget(y);

    for i in 0..N {
      z[i]
        .as_mut_ptr()
        .write(f(x0[i].as_ptr().read(), y0[i].as_ptr().read()))
    }
    mem::transmute_copy(&z)
  }
}

impl<T> Deref for Point<T> {
  type Target = [T; N];
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<T> DerefMut for Point<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

impl<T> From<(T, T)> for Point<T> {
  #[inline]
  fn from((x, y): (T, T)) -> Self {
    Self::new(x, y)
  }
}

impl<T> From<[T; N]> for Point<T> {
  #[inline]
  fn from(xs: [T; N]) -> Self {
    Self(xs)
  }
}

impl<T: Zero + Add<T, Output = T>> Zero for Point<T> {
  #[inline]
  fn zero() -> Self {
    Self(make(|| T::zero()))
  }

  #[inline]
  fn is_zero(&self) -> bool {
    self.iter().all(|x| x.is_zero())
  }
}

impl<T: Neg> Neg for Point<T> {
  type Output = Point<T::Output>;
  #[inline]
  fn neg(self) -> Self::Output {
    Point(map(self.0, |x| -x))
  }
}

impl<T: Add<U>, U> Add<Point<U>> for Point<T> {
  type Output = Point<T::Output>;
  #[inline]
  fn add(self, other: Point<U>) -> Self::Output {
    Point(zip(self.0, other.0, |x, y| x + y))
  }
}

impl<T: Sub<U>, U> Sub<Point<U>> for Point<T> {
  type Output = Point<T::Output>;
  #[inline]
  fn sub(self, other: Point<U>) -> Self::Output {
    Point(zip(self.0, other.0, |x, y| x - y))
  }
}

impl<T: Mul<U>, U: Copy> Mul<U> for Point<T> {
  type Output = Point<T::Output>;
  #[inline]
  fn mul(self, other: U) -> Self::Output {
    Point(map(self.0, |x| x * other))
  }
}

impl<T: Div<U>, U: Copy> Div<U> for Point<T> {
  type Output = Point<T::Output>;
  #[inline]
  fn div(self, other: U) -> Self::Output {
    Point(map(self.0, |x| x / other))
  }
}

impl<T: AddAssign<U>, U: Copy> AddAssign<Point<U>> for Point<T> {
  #[inline]
  fn add_assign(&mut self, other: Point<U>) {
    for (i, x) in self.iter_mut().enumerate() {
      *x += other[i];
    }
  }
}

impl<T: SubAssign<U>, U: Copy> SubAssign<Point<U>> for Point<T> {
  #[inline]
  fn sub_assign(&mut self, other: Point<U>) {
    for (i, x) in self.iter_mut().enumerate() {
      *x -= other[i];
    }
  }
}

impl<T: MulAssign<U>, U: Copy> MulAssign<U> for Point<T> {
  #[inline]
  fn mul_assign(&mut self, other: U) {
    for x in self.iter_mut() {
      *x *= other
    }
  }
}

impl<T: DivAssign<U>, U: Copy> DivAssign<U> for Point<T> {
  #[inline]
  fn div_assign(&mut self, other: U) {
    for x in self.iter_mut() {
      *x /= other
    }
  }
}

impl<T: Add<U>, U: Copy> Add<Point<U>> for Rect<T> {
  type Output = Rect<T::Output>;
  #[inline]
  fn add(self, other: Point<U>) -> Self::Output {
    Rect(self.0 + other, self.1 + other)
  }
}

impl<T: Sub<U>, U: Copy> Sub<Point<U>> for Rect<T> {
  type Output = Rect<T::Output>;
  #[inline]
  fn sub(self, other: Point<U>) -> Self::Output {
    Rect(self.0 - other, self.1 - other)
  }
}

impl<T: AddAssign<U>, U: Copy> AddAssign<Point<U>> for Rect<T> {
  #[inline]
  fn add_assign(&mut self, other: Point<U>) {
    self.0 += other;
    self.1 += other;
  }
}

impl<T: SubAssign<U>, U: Copy> SubAssign<Point<U>> for Rect<T> {
  #[inline]
  fn sub_assign(&mut self, other: Point<U>) {
    self.0 -= other;
    self.1 -= other;
  }
}
