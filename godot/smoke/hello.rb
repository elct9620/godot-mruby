puts "puts from mruby"
print "print from mruby", "\n"
p :p_from_mruby

# smoke.rb has run before this file, so the namespace it reopens has GREETING.
module Smoke
  puts GREETING
end
