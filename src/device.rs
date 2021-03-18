use core::marker::PhantomData;
use embedded_hal::serial::Read;
use embedded_hal::serial::Write;

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
    First,
    Second,
    Third,
    Fourth,
}

impl Into<u8> for SongSlot {
    fn into(self) -> u8 {
        match self {
            SongSlot::First => 0,
            SongSlot::Second => 1,
            SongSlot::Third => 2,
            SongSlot::Fourth => 3,
        }
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
        if let Err(_error) = self.write(&[128]) {
            panic!("Error entering the off state failed!");
        }
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
    pub fn send_song(&mut self, song: SongSlot) -> Result<(), <T as Write<u8>>::Error> {
        // Default two-note song for now
        self.write(&[140, song.into(), 2, 86, 64, 74, 64])?;
        Ok(())
    }
}

impl<T> Rumba<T, mode::Safe>
where
    T: Read<u8> + Write<u8>,
{
    /// Plays the specified song
    pub fn play_song(&mut self, song: SongSlot) -> Result<(), <T as Write<u8>>::Error> {
        self.write(&[141, song.into()])?;
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
