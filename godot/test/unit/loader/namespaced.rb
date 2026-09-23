module Unit
  module Loader
    # The namespace of namespaced/, which runs before the node script inside
    # it.
    module Namespaced
      GREETING = "namespace from namespaced.rb".freeze
    end
  end
end
