for %%f in (assets\vehicles\*) do ( .\target\release\ship2png.exe --ship-path %%f --parts-dir assets\parts\ --out ship_sprites\%%~nf.png -x 0.4 -g 3)
