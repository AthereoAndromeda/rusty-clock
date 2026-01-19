#!/usr/bin/env nix-shell
#!nix-shell -i nu -p brotli

def main [input?: string] {
  let content = open ($input | default ./index.html)

  print "Minifying HTML..."
  xh --body --form https://htmlcompressor.com/compress html_level=2 html_single_line=1 code=$"($content)"
    | brotli --best
    | save -f index.min.html.br  

  print "Minified and Brotli'd!"

  let og_size = du ./index.html | first | get "apparent"
  let min_size = du ./index.min.html.br | first | get "apparent"
  print $"Original Size: ($og_size)"
  print $"Minified Size: ($min_size)"
  print $"Size reduction: ($og_size - $min_size) | (100 - ($min_size / $og_size * 100) | math round -p 2)% smaller"
}
