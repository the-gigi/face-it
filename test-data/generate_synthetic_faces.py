#!/usr/bin/env python3
"""Generate synthetic test face images for integration testing.

Creates distinct faces with structural differences (not pixel noise).
Face recognition should generalize over noise but distinguish structural features.
"""

from PIL import Image, ImageDraw

def create_face(filename: str, config: dict):
    """Create a face with specific structural features.

    Args:
        filename: Output filename
        config: Dict with face parameters (skin_tone, face_shape, eyes, nose, mouth, hair)
    """
    # Always use white background - focus on structural differences only
    img = Image.new('RGB', (224, 224), color='white')
    draw = ImageDraw.Draw(img)

    # Unpack configuration
    skin_tone = config['skin_tone']
    face_shape = config['face_shape']  # (x1, y1, x2, y2)
    eyes = config['eyes']  # {'size', 'spacing', 'color', 'shape'}
    nose = config['nose']  # {'length', 'width'}
    mouth = config['mouth']  # {'y', 'width', 'expression'}
    hair = config['hair']  # {'color', 'style'}
    eyebrows = config.get('eyebrows', {})  # {'thickness', 'angle'}

    # Draw face shape
    draw.ellipse(face_shape, fill=skin_tone, outline='black', width=2)

    # Calculate face center for positioning features
    face_center_x = (face_shape[0] + face_shape[2]) // 2
    face_top = face_shape[1]

    # Draw hair
    hair_color = hair['color']
    if hair['style'] == 'short':
        draw.ellipse([face_shape[0]-5, face_top-30, face_shape[2]+5, face_top+40],
                    fill=hair_color, outline='black', width=2)
    elif hair['style'] == 'long':
        draw.ellipse([face_shape[0]-10, face_top-35, face_shape[2]+10, face_top+50],
                    fill=hair_color, outline='black', width=2)
    elif hair['style'] == 'bald':
        draw.ellipse([face_shape[0]+10, face_top-20, face_shape[2]-10, face_top+30],
                    fill=hair_color, outline='black', width=2)

    # Draw eyebrows
    brow_y = face_top + 50
    brow_thickness = eyebrows.get('thickness', 2)
    brow_angle = eyebrows.get('angle', 0)  # 0=straight, positive=worried, negative=angry
    left_brow_start = (face_center_x - eyes['spacing'] - 15, brow_y + brow_angle)
    left_brow_end = (face_center_x - eyes['spacing'] + 15, brow_y)
    right_brow_start = (face_center_x + eyes['spacing'] - 15, brow_y)
    right_brow_end = (face_center_x + eyes['spacing'] + 15, brow_y + brow_angle)
    draw.line([left_brow_start, left_brow_end], fill='black', width=brow_thickness)
    draw.line([right_brow_start, right_brow_end], fill='black', width=brow_thickness)

    # Draw eyes
    eye_y = face_top + 70
    eye_size = eyes['size']
    eye_spacing = eyes['spacing']
    eye_color = eyes['color']

    # Left eye
    left_eye_box = (face_center_x - eye_spacing - eye_size, eye_y - eye_size//2,
                    face_center_x - eye_spacing + eye_size, eye_y + eye_size//2)
    draw.ellipse(left_eye_box, fill='white', outline='black', width=2)
    # Iris
    iris_size = eye_size // 2
    draw.ellipse((face_center_x - eye_spacing - iris_size//2, eye_y - iris_size//2,
                 face_center_x - eye_spacing + iris_size//2, eye_y + iris_size//2),
                fill=eye_color, outline='black', width=1)
    # Pupil
    pupil_size = iris_size // 2
    draw.ellipse((face_center_x - eye_spacing - pupil_size//2, eye_y - pupil_size//2,
                 face_center_x - eye_spacing + pupil_size//2, eye_y + pupil_size//2),
                fill='black')

    # Right eye
    right_eye_box = (face_center_x + eye_spacing - eye_size, eye_y - eye_size//2,
                     face_center_x + eye_spacing + eye_size, eye_y + eye_size//2)
    draw.ellipse(right_eye_box, fill='white', outline='black', width=2)
    # Iris
    draw.ellipse((face_center_x + eye_spacing - iris_size//2, eye_y - iris_size//2,
                 face_center_x + eye_spacing + iris_size//2, eye_y + iris_size//2),
                fill=eye_color, outline='black', width=1)
    # Pupil
    draw.ellipse((face_center_x + eye_spacing - pupil_size//2, eye_y - pupil_size//2,
                 face_center_x + eye_spacing + pupil_size//2, eye_y + pupil_size//2),
                fill='black')

    # Draw nose
    nose_start_y = eye_y + 15
    nose_end_y = nose_start_y + nose['length']
    nose_width = nose['width']
    draw.line([(face_center_x, nose_start_y), (face_center_x, nose_end_y)],
             fill='black', width=2)
    # Nostrils
    draw.arc([face_center_x - nose_width, nose_end_y - 8,
              face_center_x + nose_width, nose_end_y + 8],
             0, 180, fill='black', width=2)

    # Draw mouth
    mouth_y = mouth['y']
    mouth_width = mouth['width']
    expression = mouth['expression']

    if expression == 'smile':
        draw.arc([face_center_x - mouth_width, mouth_y - 15,
                 face_center_x + mouth_width, mouth_y + 15],
                0, 180, fill='black', width=3)
    elif expression == 'neutral':
        draw.line([(face_center_x - mouth_width, mouth_y),
                  (face_center_x + mouth_width, mouth_y)],
                 fill='black', width=3)
    elif expression == 'frown':
        draw.arc([face_center_x - mouth_width, mouth_y - 15,
                 face_center_x + mouth_width, mouth_y + 15],
                180, 360, fill='black', width=3)

    img.save(filename)
    print(f"Created {filename}")

def main():
    print("Generating synthetic test face images with structural differences...")
    print()

    # User 1: Oval face, brown eyes, medium nose, neutral expression
    user1_config = {
        'skin_tone': (220, 180, 160),
        'face_shape': (50, 40, 174, 190),  # Oval: wider than tall
        'eyes': {
            'size': 18,
            'spacing': 35,
            'color': (101, 67, 33),  # Brown
        },
        'nose': {
            'length': 35,
            'width': 12,
        },
        'mouth': {
            'y': 155,
            'width': 30,
            'expression': 'neutral',
        },
        'hair': {
            'color': (60, 40, 20),  # Dark brown
            'style': 'short',
        },
        'eyebrows': {
            'thickness': 2,
            'angle': 0,
        }
    }
    create_face("user1.png", user1_config)

    # User 2: Round face, blue eyes, short nose, wide smile
    user2_config = {
        'skin_tone': (235, 200, 180),
        'face_shape': (40, 50, 184, 194),  # Round: equal width and height
        'eyes': {
            'size': 22,  # Larger eyes
            'spacing': 40,  # Further apart
            'color': (70, 130, 180),  # Blue
        },
        'nose': {
            'length': 28,  # Shorter nose
            'width': 15,  # Wider nose
        },
        'mouth': {
            'y': 160,  # Lower mouth
            'width': 35,  # Wider mouth
            'expression': 'smile',
        },
        'hair': {
            'color': (139, 90, 43),  # Light brown
            'style': 'long',
        },
        'eyebrows': {
            'thickness': 3,  # Thicker eyebrows
            'angle': -3,  # Slightly angry look
        }
    }
    create_face("user2.png", user2_config)

    # User 3: Long face, green eyes, long thin nose, small mouth
    user3_config = {
        'skin_tone': (140, 100, 70),  # Darker skin
        'face_shape': (60, 30, 164, 200),  # Long: taller than wide
        'eyes': {
            'size': 16,  # Smaller eyes
            'spacing': 32,  # Closer together
            'color': (34, 139, 34),  # Green
        },
        'nose': {
            'length': 42,  # Longer nose
            'width': 10,  # Thinner nose
        },
        'mouth': {
            'y': 165,  # Even lower mouth (long face)
            'width': 25,  # Narrower mouth
            'expression': 'neutral',
        },
        'hair': {
            'color': (30, 25, 20),  # Very dark/black hair
            'style': 'bald',
        },
        'eyebrows': {
            'thickness': 2,
            'angle': 2,  # Slightly worried look
        }
    }
    create_face("user3.png", user3_config)

    # User 1 Similar: Same structural features but slightly different expression
    user1_similar_config = user1_config.copy()
    user1_similar_config['mouth'] = {
        'y': 155,
        'width': 30,
        'expression': 'smile',  # Changed from neutral to smile
    }
    user1_similar_config['eyebrows'] = {
        'thickness': 2,
        'angle': -2,  # Slightly raised (happy)
    }
    create_face("user1_similar.png", user1_similar_config)

    # Different: Completely different structural features
    # Focus on STRUCTURAL differences only - no color/background tricks
    # Wide face with eyes VERY far apart, very short wide nose, mouth very low
    different_config = {
        'skin_tone': (210, 170, 150),  # Neutral skin tone (similar range to others)
        'face_shape': (35, 60, 189, 180),  # Very wide, short face (extreme aspect ratio)
        'eyes': {
            'size': 12,  # Small eyes
            'spacing': 55,  # MAXIMUM spacing - eyes very far apart
            'color': (80, 80, 80),  # Neutral gray eyes
        },
        'nose': {
            'length': 20,  # Very short nose
            'width': 20,  # Very wide nose (unusual ratio)
        },
        'mouth': {
            'y': 170,  # Very low mouth position (far from nose)
            'width': 45,  # Very wide mouth
            'expression': 'neutral',
        },
        'hair': {
            'color': (80, 60, 40),  # Neutral brown hair
            'style': 'short',
        },
        'eyebrows': {
            'thickness': 3,  # Medium thickness
            'angle': 0,
        }
    }
    create_face("different.png", different_config)

    print()
    print("✓ All test images generated successfully!")
    print()
    print("Generated images with distinct structural features:")
    print("  - user1.png: Oval face, brown eyes, medium features, neutral")
    print("  - user2.png: Round face, blue eyes, short nose, smiling")
    print("  - user3.png: Long face, green eyes, long thin nose, darker skin")
    print("  - user1_similar.png: Same as user1 but smiling (should match user1)")
    print("  - different.png: Wide face, small far-apart eyes, gray hair (should NOT match)")

if __name__ == "__main__":
    main()
