use std::sync::{Mutex, MutexGuard, PoisonError, Condvar};
use std::ops::{Deref, DerefMut};
use std::fmt;

/// Mutex with associated condvar.
/// The condvar can only be used with the associated mutex guard,
/// and only when the associated lock is held.
///
/// IE instead of (Mutex<resource>, Condvar), use CondMutex<resource>
/// This is a convenience wrapper and thus does not export
/// the entire api of Mutex. If the whole api is needed,
/// consider calling `parts()`
///
/// CondMutex also has an optional (preferably zero-sized) tag type marker,
/// to allow type-level naming of the resource guarded by the mutex.
/// For example, given a mutex-wrapped reader vec and a mutex-wrapped writer vec,
/// a reader fn could take CondMutex<Vec, READER_TAG_TYPE> to prevent
/// accidentally passing the writer vec which is otherwise identical to the typesystem.
pub type LockError<'a, T> = PoisonError<MutexGuard<'a, T>>;

#[derive(Debug, Default)]
pub struct CondMutex<T, Tag: Copy = ()> {
  _tag: Tag,
  mx: Mutex<T>,
  cv: Condvar
}

impl<T> CondMutex<T> {
  pub fn new(t: T) -> CondMutex<T> {
    CondMutex {
      _tag: (),
      mx: Mutex::new(t),
      cv: Condvar::new()
    }
  }

  pub fn parts(&self) -> (&Mutex<T>, &Condvar) {
    (&self.mx, &self.cv)
  }

  pub fn parts_mut(&mut self) -> (&mut Mutex<T>, &Condvar) {
    (&mut self.mx, &self.cv)
  }

  pub fn into_parts(self) -> (Mutex<T>, Condvar) {
    (self.mx, self.cv)
  }

  pub fn lock(&self) -> Result<CondMutexGuard<T>, LockError<T>> {
    self.mx.lock()
      .map(|guard| CondMutexGuard { _tag: self._tag, guard, cv: &self.cv })
  }
}

#[derive(Debug)]
pub struct CondMutexGuard<'a, T: ?Sized + 'a, Tag: Copy = ()> {
  _tag: Tag,
  guard: MutexGuard<'a, T>,
  cv: &'a Condvar
}

impl <'a, T> CondMutexGuard<'a, T> {
  pub fn parts(&self) -> (&MutexGuard<'a, T>, &Condvar) {
    (&self.guard, self.cv)
  }

  // NOTE: Condvar api is entirely immutable, so we only provide immutable refs
  pub fn parts_mut(&mut self) -> (&mut MutexGuard<'a, T>, &Condvar) {
    (&mut self.guard, self.cv)
  }

  pub fn into_parts(self) -> (MutexGuard<'a, T>, &'a Condvar) {
    (self.guard, self.cv)
  }

  pub fn wait(self) -> Result<CondMutexGuard<'a, T>, LockError<'a, T>> {
    let _tag = self._tag;
    let cv = self.cv;
    let guard = self.guard;
    let res = cv.wait(guard);

    res.map(|guard| CondMutexGuard { _tag, guard, cv })
  }

  pub fn notify_one(&self) {
    self.cv.notify_one()
  }

  pub fn notify_all(&self) {
    self.cv.notify_all()
  }
}

impl<T: ?Sized> Deref for CondMutexGuard<'_, T> {
  type Target = T;

  fn deref(&self) -> &T {
    self.guard.deref()
  }
}

impl<T: ?Sized> DerefMut for CondMutexGuard<'_, T> {
  fn deref_mut(&mut self) -> &mut T {
    self.guard.deref_mut()
  }
}

impl<T: ?Sized + fmt::Display> fmt::Display for CondMutexGuard<'_, T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    (**self).fmt(f)
  }
}
