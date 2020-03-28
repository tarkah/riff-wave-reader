# Riff Wave Reader

Reads riff-wave compliant files


## Using RiffWaveReader

```rust
use anyhow::Error;

use std::fs::File;
use std::io::BufReader;

use riff_wave_reader::RiffWaveReader;

fn main() -> Result<(), Error> {
    let file = File::open("path/to/file.wav")?;

    let reader = BufReader::new(file);
    let mut reader = RiffWaveReader::new(reader)?;

    // Print header info
    reader.print_info();

    let data = reader.data()?.collect::<Vec<u8>>();

    // Do stuff with data...

    Ok(())
}
```


## Print header info from CLI

```
cargo run -- print path/to/file.wav

------ Header ------
Size:            651016
Format:          ExtendedWave
Channels:        2
Sample Rate:     44100
Byte Rate:       8096
Block Align:     376
Bits per Sample: 0
Extra Info:      34
----- Extended -----
Sample Info:     2048
Channel Mask:    0b0000000000000011
Sub Format:      62cee401faff19a14471cb58e923aabf
Remaining Data:  [1, 0, 40, 46, 0, 0, 0, 0, 0, 0, 0, 0]
------- Fact -------
Fact Length:     12
Sample Length:   3541379
Remaining Data:  [214, 9, 0, 0, 142, 10, 0, 0]
--- Other Chunks ---
Chunk Ids:       ["smpl"]
------- Data -------
Data Length:     650856
Padding Byte:    0
```