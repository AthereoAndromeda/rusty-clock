#!/usr/bin/env nix-shell
#!nix-shell -i nu -p brotli

def main [input?: path, output?: path] {
  let input_path = ($input | default ./index.html)
  let output_path = ($output | default ./index.min.html.br)
  
  let content = open $input_path

  print "Minifying HTML..."
  xh --body --form https://htmlcompressor.com/compress html_level=2 html_single_line=1 code=$"($content)"
    | brotli --best
    | save -f $output_path

  print "Minified and Brotli'd!"

  let og_size = du $input_path | first | get "apparent"
  let min_size = du $output_path | first | get "apparent"
  print $"Original Size: ($og_size)"
  print $"Minified Size: ($min_size)"
  print $"Size reduction: ($og_size - $min_size) | (100 - ($min_size / $og_size * 100) | math round -p 2)% smaller"
}
