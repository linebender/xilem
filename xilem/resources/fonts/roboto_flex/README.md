# Roboto Flex Subset

Roboto Flex is a variable font, which is used to validate Masonry's support for variable fonts.
This is used in the `variable_clock` example, and has a subset chosen to also be usable by the `calc` example (although it is not currently used there).

Roboto Flex is a variable font with these axes:
  GRAD
  XOPQ
  XTRA
  YOPQ
  YTAS
  YTDE
  YTFI
  YTLC
  YTUC
  opsz
  slnt
  wdth
  wght

The subset was created using the command:

```shell
> fonttools subset --text="0123456789:-/+=÷×±()" --output-file=RobotoFlex-Subset.ttf RobotoFlex-VariableFont_GRAD,XOPQ,XTRA,YOPQ,YTAS,YTDE,YTFI,YTLC,YTUC,opsz,slnt,wdth,wght.ttf --layout-features='*'
```

with RobotoFlex [downloaded from Google Fonts](https://fonts.google.com/specimen/Roboto+Flex) in early August 2024:

```shell
> sha256hmac RobotoFlex-VariableFont_GRAD,XOPQ,XTRA,YOPQ,YTAS,YTDE,YTFI,YTLC,YTUC,opsz,slnt,wdth,wght.ttf 
9efabb32d95826786ba339a43b6e574660fab1cad4dbf9b9e5e45aaef9d1eec3  RobotoFlex-VariableFont_GRAD,XOPQ,XTRA,YOPQ,YTAS,YTDE,YTFI,YTLC,YTUC,opsz,slnt,wdth,wght.ttf
```

## License

RobotoFlex-Subset.ttf: Copyright 2017 The Roboto Flex Project Authors (<https://github.com/TypeNetwork/Roboto-Flex>)

Please read the full license text ([OFL.txt](./OFL.txt)) to understand the permissions,
restrictions and requirements for usage, redistribution, and modification.
