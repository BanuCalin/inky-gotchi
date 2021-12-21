use embedded_hal as hal;

/// Populates data buffer (array) and returns a pair (tuple) with command and
/// appropriately sized slice into populated buffer.
/// E.g.
///
/// let mut buf = [0u8; 4];
/// let (command, data) = pack!(buf, 0x3C, [0x12, 0x34]);
macro_rules! pack {
    ($buf:ident, $cmd:expr,[]) => {
        ($cmd, &$buf[..0])
    };
    ($buf:ident, $cmd:expr,[$arg0:expr]) => {{
        $buf[0] = $arg0;
        ($cmd, &$buf[..1])
    }};
    ($buf:ident, $cmd:expr,[$arg0:expr, $arg1:expr]) => {{
        $buf[0] = $arg0;
        $buf[1] = $arg1;
        ($cmd, &$buf[..2])
    }};
    ($buf:ident, $cmd:expr,[$arg0:expr, $arg1:expr, $arg2:expr]) => {{
        $buf[0] = $arg0;
        $buf[1] = $arg1;
        $buf[2] = $arg2;
        ($cmd, &$buf[..3])
    }};
    ($buf:ident, $cmd:expr,[$arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr]) => {{
        $buf[0] = $arg0;
        $buf[1] = $arg1;
        $buf[2] = $arg2;
        $buf[3] = $arg3;
        ($cmd, &$buf[..4])
    }};
}

mod ssd1675 {
    use embedded_hal as hal;
    use core::fmt::Debug;

    const RESET_DELAY_MS: u8 = 10;
    const MAX_GATES: u16 = 296;
    pub const MAX_SOURCE_OUTPUTS: u8 = 160;
    pub const MAX_DUMMY_LINE_PERIOD: u8 = 127;
    const ANALOG_BLOCK_CONTROL_MAGIC: u8 = 0x54;
    const DIGITAL_BLOCK_CONTROL_MAGIC: u8 = 0x3B;

    pub struct Interface<SPI, CS, BUSY, DC, RESET> {
        /// SPI interface
        spi: SPI,
        /// CS (chip select) for SPI (output)
        cs: CS,
        /// Active low busy pin (input)
        busy: BUSY,
        /// Data/Command Control Pin (High for data, Low for command) (output)
        dc: DC,
        /// Pin for reseting the controller (output)
        reset: RESET,
    }

    #[derive(Clone, Copy)]
    pub enum IncrementAxis {
        /// X direction
        Horizontal,
        /// Y direction
        Vertical,
    }

    #[derive(Clone, Copy)]
    pub enum DataEntryMode {
        DecrementXDecrementY,
        IncrementXDecrementY,
        DecrementXIncrementY,
        IncrementYIncrementX, // POR
    }

    #[derive(Clone, Copy)]
    pub enum TemperatureSensor {
        Internal,
        External,
    }
    
    #[derive(Clone, Copy)]
    pub enum RamOption {
        Normal,
        Bypass,
        Invert,
    }

    #[derive(Clone, Copy)]
    pub enum DeepSleepMode {
        /// Not sleeping
        Normal,
        /// Deep sleep with RAM preserved
        PreserveRAM,
        /// Deep sleep RAM not preserved
        DiscardRAM,
    }

    pub enum Command {
        /// Set the MUX of gate lines, scanning sequence and direction
        /// 0: MAX gate lines
        /// 1: Gate scanning sequence and direction
        DriverOutputControl,
        /// Set the gate driving voltage.
        GateDrivingVoltage,
        /// Set the source driving voltage.
        /// 0: VSH1
        /// 1: VSH2
        /// 2: VSL
        SourceDrivingVoltage,
        /// Booster enable with phases 1 to 3 for soft start current and duration setting
        /// 0: Soft start setting for phase 1
        /// 1: Soft start setting for phase 2
        /// 2: Soft start setting for phase 3
        /// 3: Duration setting
        BoosterEnable,
        /// Set the scanning start position of the gate driver
        GateScanStartPostion,
        /// Set deep sleep mode
        DeepSleepMode,
        /// Set the data entry mode and increament axis
        DataEntryMode,
        /// Perform a soft reset, and reset all parameters to their default values
        /// BUSY will be high when in progress.
        SoftReset,
        // /// Start HV ready detection. Read result with `ReadStatusBit` command
        // StartHVReadyDetection,
        // /// Start VCI level detection
        // /// 0: threshold
        // /// Read result with `ReadStatusBit` command
        // StartVCILevelDetection(u8),
        /// Specify internal or external temperature sensor
        TemperatatSensorSelection,
        /// Write to the temperature sensor register
        WriteTemperatureSensor,
        /// Read from the temperature sensor register
        ReadTemperatureSensor,
        /// Write a command to the external temperature sensor
        WriteExternalTemperatureSensor,
        /// Activate display update sequence. BUSY will be high when in progress.
        UpdateDisplay,
        /// Set RAM content options for update display command.
        /// 0: Black/White RAM option
        /// 1: Color RAM option
        UpdateDisplayOption1,
        /// Set display update sequence options
        UpdateDisplayOption2,
        // Read from RAM (not implemented)
        // ReadData,
        /// Enter VCOM sensing and hold for duration defined by VCOMSenseDuration
        /// BUSY will be high when in progress.
        EnterVCOMSensing,
        /// Set VCOM sensing duration
        VCOMSenseDuration,
        // /// Program VCOM register into OTP
        // ProgramVCOMIntoOTP,
        /// Write VCOM register from MCU interface
        WriteVCOM,
        // ReadDisplayOption,
        // ReadUserId,
        // StatusBitRead,
        // ProgramWaveformSetting,
        // LoadWaveformSetting,
        // CalculateCRC,
        // ReadCRC,
        // ProgramOTP,
        // WriteDisplayOption,
        // WriteUserId,
        // OTPProgramMode,
        /// Set the number of dummy line period in terms of gate line width (TGate)
        DummyLinePeriod,
        /// Set the gate line width (TGate)
        GateLineWidth,
        /// Select border waveform for VBD
        BorderWaveform,
        // ReadRamOption,
        /// Set the start/end positions of the window address in the X direction
        /// 0: Start
        /// 1: End
        StartEndXPosition,
        /// Set the start/end positions of the window address in the Y direction
        /// 0: Start
        /// 1: End
        StartEndYPosition,
        /// Auto write Color RAM for regular pattern
        AutoWriteColorPattern,
        /// Auto write Black RAM for regular pattern
        AutoWriteBlackPattern,
        /// Set RAM X address
        XAddress,
        /// Set RAM Y address
        YAddress,
        /// Set analog block control
        AnalogBlockControl,
        /// Set digital block control
        DigitalBlockControl,
        // Used to terminate frame memory reads
        // Nop,
    
        WriteBlackData,
        /// Write to Color RAM
        /// 1 = Color
        /// 0 = Use contents of black/white RAM
        WriteColorData,
        /// Write LUT register (70 bytes)
        WriteLUT,
    }

    union ComandData<'buf> {
        driver_output_control: (u16, u8),
        gate_driving_voltage: u8,
        source_driving_voltage: (u8, u8, u8),
        booster_enable: (u8, u8, u8, u8),
        gate_scan_start_position: u16,
        deep_sleep_mode: DeepSleepMode,
        data_entry_mode: (DataEntryMode, IncrementAxis),
        temperature_sensor_selection: TemperatureSensor,
        write_temperature_sensor: u16,
        read_temperature_sensor: u16,
        write_external_temperature_sensor: (u8, u8, u8),
        update_display_option_1: (RamOption, RamOption),
        update_display_option_2: u8,
        vcom_sense_duration: u8,
        write_vcom: u8,
        dummy_line_period: u8,
        gate_line_width: u8,
        border_wave_form: u8,
        start_end_x_position: (u8, u8),
        start_end_y_position: (u16, u16),
        auto_write_color_pattern: u8,
        auto_write_black_pattern: u8,
        x_address: u8,
        y_address: u8,
        analog_block_control: u8,
        digital_block_control: u8,

        write_black_data: &'buf [u8],
        write_color_data: &'buf [u8],
        write_lut: &'buf [u8],
    }

    impl<SPI, CS, BUSY, DC, RESET> Interface<SPI, CS, BUSY, DC, RESET>
    where
        SPI: hal::blocking::spi::Write<u8>,
        CS: hal::digital::v2::OutputPin,
        CS::Error: Debug,
        BUSY: hal::digital::v2::InputPin,
        DC: hal::digital::v2::OutputPin,
        DC::Error: Debug,
        RESET: hal::digital::v2::OutputPin,
        RESET::Error: Debug,
    {
        /// Create a new Interface from embedded hal traits.
        pub fn new(spi: SPI, cs: CS, busy: BUSY, dc: DC, reset: RESET) -> Self {
            Self {
                spi,
                cs,
                busy,
                dc,
                reset,
            }
        }

        fn write(&mut self, data: &[u8]) -> Result<(), SPI::Error> {
            // Select the controller with chip select (CS)
            // self.cs.set_low();
    
            // Linux has a default limit of 4096 bytes per SPI transfer
            // https://github.com/torvalds/linux/blob/ccda4af0f4b92f7b4c308d3acc262f4a7e3affad/drivers/spi/spidev.c#L93
            if cfg!(target_os = "linux") {
                for data_chunk in data.chunks(4096) {
                    self.spi.write(data_chunk)?;
                }
            } else {
                self.spi.write(data)?;
            }
    
            // Release the controller
            // self.cs.set_high();
    
            Ok(())
        }

        pub fn reset<D: hal::blocking::delay::DelayMs<u8>>(&mut self, delay: &mut D) {
            self.reset.set_low().unwrap();
            delay.delay_ms(RESET_DELAY_MS);
            self.reset.set_high().unwrap();
            delay.delay_ms(RESET_DELAY_MS);
        }

        pub fn send_command(&mut self, command: Command, command_data: ComandData)-> Result<(), SPI::Error> {
            let mut buf = [0u8; 4];
            unsafe {
                let (command, data) = match command {
                    Command::DriverOutputControl => {
                        let [upper, lower] = command_data.driver_output_control.0.to_be_bytes();
                        pack!(buf, 0x01, [lower, upper, command_data.driver_output_control.1])
                    }
                    GateDrivingVoltage => pack!(buf, 0x03, [command_data.gate_driving_voltage]),
                    SourceDrivingVoltage => pack!(buf, 0x04, [
                        command_data.source_driving_voltage.0,
                        command_data.source_driving_voltage.1,
                        command_data.source_driving_voltage.2]),
                    BoosterEnable => pack!(buf, 0x0C, [
                        command_data.booster_enable.0, 
                        command_data.booster_enable.1,
                        command_data.booster_enable.2,
                        command_data.booster_enable.3]),
                    GateScanStartPostion => {
                        // debug_assert!(Contains::contains(&(0..MAX_GATES), position));
                        let [upper, lower] = command_data.gate_scan_start_position.to_be_bytes();
                        pack!(buf, 0x0F, [lower, upper])
                    }
                    DeepSleepMode => {
                        let mode = match command_data.deep_sleep_mode {
                            self::DeepSleepMode::Normal => 0b00,
                            self::DeepSleepMode::PreserveRAM => 0b01,
                            self::DeepSleepMode::DiscardRAM => 0b11,
                        };
        
                        pack!(buf, 0x10, [mode])
                    }
                    DataEntryMode => {
                        let mode = match command_data.data_entry_mode.0 {
                            self::DataEntryMode::DecrementXDecrementY => 0b00,
                            self::DataEntryMode::IncrementXDecrementY => 0b01,
                            self::DataEntryMode::DecrementXIncrementY => 0b10,
                            self::DataEntryMode::IncrementYIncrementX => 0b11,
                        };
                        let axis = match command_data.data_entry_mode.1 {
                            IncrementAxis::Horizontal => 0b000,
                            IncrementAxis::Vertical => 0b100,
                        };
        
                        pack!(buf, 0x11, [axis | mode])
                    }
                    SoftReset => pack!(buf, 0x12, []),
                    // TemperatatSensorSelection(TemperatureSensor) => {
                    // }
                    // WriteTemperatureSensor(u16) => {
                    // }
                    // ReadTemperatureSensor(u16) => {
                    // }
                    // WriteExternalTemperatureSensor(u8, u8, u8) => {
                    // }
                    UpdateDisplay => pack!(buf, 0x20, []),
                    // UpdateDisplayOption1(RamOption, RamOption) => {
                    // }
                    UpdateDisplayOption2 => pack!(buf, 0x22, [command_data.update_display_option_2]),
                    // EnterVCOMSensing => {
                    // }
                    // VCOMSenseDuration(u8) => {
                    // }
                    WriteVCOM => pack!(buf, 0x2C, [command_data.vcom_sense_duration]),
                    DummyLinePeriod => {
                        // debug_assert!(Contains::contains(&(0..=MAX_DUMMY_LINE_PERIOD), period));
                        pack!(buf, 0x3A, [command_data.dummy_line_period])
                    }
                    GateLineWidth => pack!(buf, 0x3B, [command_data.gate_line_width]),
                    BorderWaveform => pack!(buf, 0x3C, [command_data.border_wave_form]),
                    StartEndXPosition => pack!(buf, 0x44, [
                        command_data.start_end_x_position.0,
                        command_data.start_end_x_position.1]),
                    StartEndYPosition => {
                        let [start_upper, start_lower] =
                            command_data.start_end_y_position.0.to_be_bytes();
                        let [end_upper, end_lower] =
                            command_data.start_end_y_position.1.to_be_bytes();
                        pack!(buf, 0x45, [start_lower, start_upper, end_lower, end_upper])
                    }
                    // AutoWriteRedPattern(u8) => {
                    // }
                    // AutoWriteBlackPattern(u8) => {
                    // }
                    XAddress => pack!(buf, 0x4E, [command_data.x_address]),
                    YAddress => pack!(buf, 0x4F, [command_data.y_address]),
                    AnalogBlockControl => pack!(buf, 0x74, [command_data.analog_block_control]),
                    DigitalBlockControl => pack!(buf, 0x7E, [command_data.digital_block_control]),
                    WriteBlackData => (0x24, command_data.write_black_data),
                    WriteColorData => (0x26, command_data.write_color_data),
                    WriteLUT => (0x32, command_data.write_lut),
                    _ => unimplemented!(),
                };

                self.send_command_code(command)?;
                if data.len() == 0 {
                    Ok(())
                } else {
                    self.send_data(data)
                }
            }
        }

        fn send_command_code(&mut self, command: u8) -> Result<(), SPI::Error> {
            self.dc.set_low().unwrap();
            self.write(&[command])?;
            self.dc.set_high().unwrap();
    
            Ok(())
        }
    
        fn send_data(&mut self, data: &[u8]) -> Result<(), SPI::Error> {
            self.dc.set_high().unwrap();
            self.write(data)
        }
    
        pub fn busy_wait(&self) {
            while match self.busy.is_high() {
                Ok(x) => x,
                _ => false,
            } {}
        }

    }
}

mod Display {
    use std::fmt::Error;


    pub enum Rotation {
        Rotate0,
        Rotate90,
        Rotate180,
        Rotate270,
    }

    struct Display<'a> {
        dummy_line_period: u8,
        gate_line_width: u8,
        write_vcom: u8,
        rotation: Rotation,
        lut: Option<&'a [u8]>,
        rows: u16,
        cols: u16,
    }

    impl<'a> Display<'a> {
        pub fn vcom(&mut self, value: u8) -> Result<(), Error> {
            self.write_vcom = value;
            Ok(())
        }

        pub fn dimensions(&mut self, rows: u16, cols: u16) -> Result<(), Error> {
            assert!(
                cols % 8 == 0,
                "columns must be evenly divisible by 8"
            );
            assert!(
                rows <= ssd1675::MAX_GATE_OUTPUTS,
                "rows must be less than MAX_GATE_OUTPUTS"
            );
            assert!(
                cols <= ssd1675::MAX_SOURCE_OUTPUTS,
                "cols must be less than MAX_SOURCE_OUTPUTS"
            );
    
            self.rows = rows;
            self.cols = cols;
            Ok(())
        }

        pub fn rotation(&mut self, rotation: Rotation)  {
            self.rotation = rotation;
        }

        pub fn lut(self, lut: &'a [u8]) {
           self.lut = Some(lut);
        }
    }
}
fn main() {

}