# namespaced.rb has run before this file, so the namespace it reopens has
# GREETING.
module Verify
  module Loader
    module Namespaced
      puts GREETING
    end
  end
end
