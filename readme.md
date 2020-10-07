# Padantic
Fast as fuck CLI tool for exploiting ALL sorts of padding oracles.

## Optimizations

- Block level multithreading
- Guesses follow a priority list which is initialized to english letter frequencies and automatically readjusted as new bytes get found
- Last PAD bytes take a maximum of BLOCKSIZE tries

## Usage
The oracle - in this case - is actually a command you supply,
which takes the base64 encoded block via stdin and writes yes (correct padding)/no (incorrect padding) to stdout. You can find an example python oracle in [python example](https://github.com/sum-catnip/padantic/blob/master/oracle_example.py). Oracles can be written in any language (or just make a shell script). The full oracle command line will get executed so feel free pass command line options.

usual usage:
> padantic [hex encoded ciphertext] -O out -- python oracle.py
 
 example usage:
 > padantic 4B730C0BC9F1FCA944D50B012DCDCDCD58D0BADA3B0CDADD849EFFB5351B1EE55A1D168B337089F5A88E43D4A7F403C5E527F9CAF5825A88B3E4EC72A2FEE1230FA71DA08C71ACB58D663679E265213567341CA9918AA14A8D6983D65C39E463560859BFA290DAFC419679CCA9688A7EF377E5E095B1D5876A50E90EB7DA9487CB8C141F15AF61431E3DB139DF3D8370 -O out -- python oracle.py
 
 Command line Arguments:
```
padantic 0.1.0
catnip <catnip@catnip.fyi>


USAGE:
    padantic [FLAGS] [OPTIONS] <cipher> <oracle>...

FLAGS:
    -h, --help       
            Prints help information

    -v               
            use multiple times to increase log level. ex: -vv for `info`. 1 is the default so errors are always logged

        --noiv       
            skip CBC on first block and guess IV interactively

    -V, --version    
            Prints version information


OPTIONS:
        --chars <chars>      
            (space seperated) list of hex encoded bytes to guess the plaintext. ALL 256 POSSIBLE BYTES MUST BE PRESENT
            in no particular order. example: 00 01 02 ... 61 62 63 ... 6A 6B ... FF FF [default: english.chars]
        --log <log>          
            target file for logging output. log will contain stderr output from the oracle.

    -O, --output <output>    
            writes result to file

    -s, --size <size>        
            CBC block size [default: 16]


ARGS:
    <cipher>       
            target ciphertext (hex encoded)

    <oracle>...    
            the command to run as an oracle. should only return status 0 for valid padding. command will be ran with
            base64 paylod as first arg. arguments after cmd argument will be prepended BEFORE payload
```

## Installation
> cargo install padantic

To run it you must have the cargo bin directory in your path (recommended) or write
> cargo run padantic

instead of just `padantic`

## Look how pretty it looks
I'll need to update this gif someday since ive added colors to padantic but anyways, enjoy this gif:
![animated gif](https://cdn.discordapp.com/attachments/567308861640540210/687588463721447470/speed-oracle.gif)


## Tricks
- The -noiv tags gets you the first blocks intermediate bytes so you can recover the IV if you guess the first blocks plaintext by simple XOR'ing them
- Consider adding a random delay (and useragent) to your oracle
- The -O tag is optional but its very usefull to have the results stored in a file
- If you need to debug your oracle, write to stderr and use the --log switch to generate a logfile. The logfile will contain every stderr output
