use core::marker::PhantomData;
use embedded_hal::serial::Read;
use embedded_hal::serial::Write;

/// Rumba can perform certain actions only in certain modes. This module contains the possible
/// modes the Rumba can belong to.
pub mod mode {
    /// Rumba starts in the Off mode and transitions to Passive when the start is requested.
    pub struct Off;
    /// In passive mode we are able to inspect the state of the sensors, but have no control
    /// over the Rumba. Remember to switch back to the Off state once you're done to preserve
    /// battery.
    pub struct Passive;
    /// In Safe mode we have control over the actuators, but there are still some safety sensors
    /// that are able to revert the Rumba back to its Passive mode. Remember to switch back to the
    /// Off state once you're done to preserve battery.
    pub struct Safe;
    /// In Full mode we have complete control over the Rumba. In this mode even the safety sensors
    /// do not switch back to Passive or Safe. Remember to switch back to the
    /// Off state once you're done to preserve battery.
    pub struct Full;
}

/// Possible song slots for the Rumba.
/// Selects the slot that will be used to store/play the song.
#[derive(Clone, Copy)]
pub enum SongSlot {
    First = 0,
    Second = 1,
    Third = 2,
    Fourth = 3,
}

/// Represents the duration of a single note in ticks of 1/64 seconds. You can construct an
/// instance of this structure using the U16Ext trait for u16 in the prelude:
/// ```rust
/// use rumba::{NoteDuration, prelude::*};
///
/// let duration: NoteDuration = 64u16.ms();
/// ```
pub struct NoteDuration {
    ticks: u8,
}

/// Common traits for the Rumba. This includes an extension for u16 to convert it into milliseconds
/// for the Rumba timebase
pub mod prelude {
    use super::NoteDuration;

    pub trait U16Ext {
        fn ms(self) -> NoteDuration;
    }

    impl U16Ext for u16 {
        fn ms(self) -> NoteDuration {
            let ticks = (self as u64 * 64 / 1000) as u8;
            NoteDuration { ticks }
        }
    }
}

/// Octave of a note. An octave marks the difference between a note and another of double the
/// frequency. Each of these octaves represent a range of frequencies.
#[derive(Clone, Copy)]
pub enum NoteOctave {
    Silent = 0,
    Contra = 24,
    Great = 36,
    Small = 48,
    OneLined = 60,
    TwoLined = 72,
    ThreeLined = 84,
    FourLined = 96,
}

/// Name of the note. It does not identify the freqnency of the note, since it does not have an
/// associated octave.
#[derive(Clone, Copy)]
pub enum NoteName {
    C = 0,
    CSharp = 1,
    D = 2,
    DSharp = 3,
    E = 4,
    F = 5,
    FSharp = 6,
    G = 7,
    GSharp = 8,
    A = 9,
    ASharp = 10,
    B = 11,
}

/// Complete note representation. It has an associated octave and name and duration.
pub struct Note {
    pub name: NoteName,
    pub octave: NoteOctave,
    pub duration: NoteDuration,
}

impl Note {
    fn duration(&self) -> u8 {
        self.duration.ticks
    }

    fn midi_value(&self) -> u8 {
        self.name as u8 + self.octave as u8
    }
}

/// Representation of a Roomba instance.
pub struct Rumba<T: Read<u8> + Write<u8>, MODE> {
    io_port: Option<T>,
    _mode: PhantomData<MODE>,
}

impl<T> Rumba<T, mode::Off>
where
    T: Read<u8> + Write<u8>,
{
    /// Constructs a roomba from the given serial port in the Off state
    pub fn new(io_port: T) -> Self {
        Rumba {
            io_port: Some(io_port),
            _mode: PhantomData,
        }
    }

    /// Switches to the Passive state
    pub fn into_passive(mut self) -> Rumba<T, mode::Passive> {
        self.enter_passive_state();
        Rumba {
            io_port: Some(self.decompose()),
            _mode: PhantomData,
        }
    }
}

impl<T> Rumba<T, mode::Passive>
where
    T: Read<u8> + Write<u8>,
{
    /// Switches to the Off state
    pub fn into_off(mut self) -> Rumba<T, mode::Off> {
        self.enter_off_state();
        Rumba {
            io_port: Some(self.decompose()),
            _mode: PhantomData,
        }
    }

    /// Switches to the Safe state
    pub fn into_safe(mut self) -> Rumba<T, mode::Safe> {
        if let Err(_error) = self.write(&[131]) {
            panic!("Error entering the off state failed!");
        }
        Rumba {
            io_port: Some(self.decompose()),
            _mode: PhantomData,
        }
    }

    /// Sends a predefined song to the Rumba at the specified slot
    pub fn send_song(
        &mut self,
        song: SongSlot,
        notes: &[Note],
    ) -> Result<(), <T as Write<u8>>::Error> {
        let mut buffer = [0; 35];
        buffer[0] = 140;
        buffer[1] = song as u8;
        buffer[2] = notes.len() as u8;
        for (index, element) in notes.iter().enumerate() {
            buffer[3 + index * 2] = element.midi_value();
            buffer[4 + index * 2] = element.duration();
        }
        self.write(&buffer[..3 + 2 * notes.len()])?;
        Ok(())
    }

    /// Starts/stops the cleaning mode in Rumba
    pub fn clean(&mut self) -> Result<(), <T as Write<u8>>::Error> {
        self.write(&[135])?;
        Ok(())
    }

    /// Starts cleaning in max mode
    pub fn max_clean(&mut self) -> Result<(), <T as Write<u8>>::Error> {
        self.write(&[136])?;
        Ok(())
    }
}

impl<T> Rumba<T, mode::Safe>
where
    T: Read<u8> + Write<u8>,
{
    /// Plays the specified song
    pub fn play_song(&mut self, song: SongSlot) -> Result<(), <T as Write<u8>>::Error> {
        self.write(&[141, song as u8])?;
        Ok(())
    }

    /// Switches to the Off state
    pub fn into_off(mut self) -> Rumba<T, mode::Off> {
        self.enter_off_state();
        Rumba {
            io_port: Some(self.decompose()),
            _mode: PhantomData,
        }
    }

    /// Switches to the Passive state
    pub fn into_passive(mut self) -> Rumba<T, mode::Passive> {
        self.enter_passive_state();
        Rumba {
            io_port: Some(self.decompose()),
            _mode: PhantomData,
        }
    }
}

impl<T, MODE> Rumba<T, MODE>
where
    T: Read<u8> + Write<u8>,
{
    fn write(&mut self, buffer: &[u8]) -> Result<(), <T as Write<u8>>::Error> {
        for element in buffer {
            nb::block!(self.io_port.as_mut().unwrap().write(*element))?;
        }
        Ok(())
    }

    fn enter_off_state(&mut self) {
        if let Err(_error) = self.write(&[173]) {
            panic!("Error entering the off state failed!");
        }
    }

    fn enter_passive_state(&mut self) {
        if let Err(_error) = self.write(&[128]) {
            panic!("Error entering the passive state failed!");
        }
    }

    fn decompose(self) -> T {
        let mut roomba = core::mem::ManuallyDrop::new(self);
        roomba.io_port.take().unwrap()
    }
}

impl<T, MODE> Drop for Rumba<T, MODE>
where
    T: Read<u8> + Write<u8>,
{
    fn drop(&mut self) {
        self.enter_off_state();
    }
}

#[cfg(test)]
mod tests {
    use super::prelude::U16Ext;
    use super::*;

    extern crate std;

    use std::assert_eq;
    use std::cell::RefCell;
    use std::vec;
    use std::vec::Vec;

    struct MockSerial<'a> {
        data: &'a RefCell<Vec<u8>>,
    }

    impl<'a> Write<u8> for MockSerial<'a> {
        type Error = core::convert::Infallible;
        fn write(&mut self, value: u8) -> nb::Result<(), Self::Error> {
            self.data.borrow_mut().push(value);
            Ok(())
        }
        fn flush(&mut self) -> nb::Result<(), Self::Error> {
            Ok(())
        }
    }

    impl<'a> Read<u8> for MockSerial<'a> {
        type Error = core::convert::Infallible;
        fn read(&mut self) -> nb::Result<u8, Self::Error> {
            Ok(0)
        }
    }

    #[test]
    fn rumba_is_droped() {
        let vector = std::cell::RefCell::new(std::vec![]);
        let serial = MockSerial { data: &vector };
        {
            let _ = Rumba::new(serial);
        }
        assert_eq!(*vector.borrow(), vec![173]);
    }

    #[test]
    fn rumba_enters_passive() {
        let vector = std::cell::RefCell::new(std::vec![]);
        let serial = MockSerial { data: &vector };
        {
            let rumba = Rumba::new(serial);
            assert_eq!(*vector.borrow(), vec![]);
            let _rumba = rumba.into_passive();
            assert_eq!(*vector.borrow(), vec![128]);
            vector.borrow_mut().clear();
        }
        assert_eq!(*vector.borrow(), vec![173]);
    }

    #[test]
    fn rumba_sends_song() {
        let vector = std::cell::RefCell::new(std::vec![]);
        let serial = MockSerial { data: &vector };
        {
            let rumba = Rumba::new(serial);
            assert_eq!(*vector.borrow(), vec![]);
            let mut rumba = rumba.into_passive();
            assert_eq!(*vector.borrow(), vec![128]);
            vector.borrow_mut().clear();

            let song = [
                Note {
                    name: NoteName::D,
                    duration: 1000.ms(),
                    octave: NoteOctave::ThreeLined,
                },
                Note {
                    name: NoteName::D,
                    duration: 1000.ms(),
                    octave: NoteOctave::TwoLined,
                },
            ];

            rumba.send_song(SongSlot::First, &song).unwrap();
            assert_eq!(*vector.borrow(), vec![140, 0, 2, 86, 64, 74, 64]);
            vector.borrow_mut().clear();
        }
        assert_eq!(*vector.borrow(), vec![173]);
    }

    #[test]
    fn rumba_enters_safe() {
        let vector = std::cell::RefCell::new(std::vec![]);
        let serial = MockSerial { data: &vector };

        let rumba = Rumba::new(serial);
        assert_eq!(*vector.borrow(), vec![]);

        let rumba = rumba.into_passive();
        assert_eq!(*vector.borrow(), vec![128]);
        vector.borrow_mut().clear();

        let _rumba = rumba.into_safe();
        assert_eq!(*vector.borrow(), vec![131]);
    }

    #[test]
    fn rumba_back_to_passive() {
        let vector = std::cell::RefCell::new(std::vec![]);
        let serial = MockSerial { data: &vector };

        let rumba = Rumba::new(serial);
        assert_eq!(*vector.borrow(), vec![]);

        let rumba = rumba.into_passive();
        assert_eq!(*vector.borrow(), vec![128]);
        vector.borrow_mut().clear();

        let rumba = rumba.into_safe();
        assert_eq!(*vector.borrow(), vec![131]);
        vector.borrow_mut().clear();

        let _rumba = rumba.into_passive();
        assert_eq!(*vector.borrow(), vec![128]);
    }

    #[test]
    fn note_duration_from_ms() {
        assert_eq!(16u16.ms().ticks, 1);
    }
}
