module Unit
  module Script
    # A node script whose source a test changes in Godot after the file has
    # run, without running it again, as the editor does while it is typed.
    class Tuned < Godot::Node2D
      TREBLE = 3
      NOTES = %i[pitch].freeze

      signal :hummed
      signal :"#{:str}ummed"
      signal :sung, *NOTES

      export :bass, 1
      export :treble, TREBLE
      export :"#{:mid}dle", 2
    end
  end
end
