"""Share a complete text example using the playground's existing fragment format."""
from base64 import urlsafe_b64encode
from urllib.parse import urlencode


def playground_url(source, language, width_mm, height_mm, dpmm):
    encoded = urlsafe_b64encode(source.encode('utf-8')).decode('ascii').rstrip('=')
    # The historic permalink stores inches even when displaying millimetres.
    settings = {
        'f': language, 'w': width_mm / 25.4, 'h': height_mm / 25.4,
        'u': 'mm', 'd': dpmm, 'm': 'raster', 's': 1, 'p': 'first',
        'a': 0, 'c': 'b' + encoded,
    }
    return '/playground/#' + urlencode(settings)
