import { Modal } from '@mantine/core'

export default function HelpModal({ opened, onClose }: { opened: boolean; onClose: () => void }) {
  return <Modal opened={opened} onClose={onClose} title="About ViewFinder" centered size={720} radius={15} zIndex={2000}
    closeButtonProps={{ 'aria-label': 'Close help' }}
    classNames={{ content: 'help-modal', header: 'help-header', title: 'help-title', body: 'help-body' }}>
    <p className="help-intro">Explore the view from a point in Vancouver. ViewFinder uses LiDAR surface data to estimate sightlines, render a panorama, and compare the features around your chosen location.</p>

    <section aria-labelledby="help-start"><h2 id="help-start">Getting started</h2>
      <p>Click the map or search for an address to select a location. The shaded area shows the estimated view at eye level. Drag the panorama, or focus it and use the arrow keys, to look around. Switch between the street map and satellite imagery for context.</p>
      <p>The view metrics and panorama load separately from the AI description. You can explore while the description is loading; use Read more for longer descriptions and retry if a request fails.</p>
    </section>

    <section aria-labelledby="help-data"><h2 id="help-data">Datasets & services</h2>
      <dl className="help-definitions">
        <div><dt>Vancouver LiDAR</dt><dd>Surface elevations and classification labels for ground, vegetation, buildings and water. The project’s export uses a 0.5 m grid, keeping the highest surface per cell after filtering noise and spikes. Ocean cells are flattened to 0 m in the export, which can also flatten bridges over water.</dd></div>
        <div><dt>Ocean boundaries</dt><dd>Marine Regions World Internal Waters, version 4. Canadian polygons are used to calculate overlap with the view area and to mask ocean cells during LiDAR export.</dd></div>
        <div><dt>Lakes & landmarks</dt><dd>The project’s Vancouver lake polygons and named landmark geometries. These layers provide water area and landmark matches within the calculated view polygon.</dd></div>
        <div><dt>Maps & address search</dt><dd>CARTO Voyager street maps use OpenStreetMap data. Satellite imagery comes from Esri World Imagery and its contributors. Address search uses Photon with results limited to Vancouver and its immediate surroundings.</dd></div>
        <div><dt>Sun & surroundings</dt><dd>Astral computes sunrise and sunset for the location and current date in Vancouver time. A Gemini model describes the synthetic LiDAR panorama; it does not supply the numeric view metrics.</dd></div>
      </dl>
    </section>

    <section aria-labelledby="help-metrics"><h2 id="help-metrics">How the view is computed</h2>
      <dl className="help-definitions">
        <div><dt>Eye level & sightlines</dt><dd>The observer is placed 2.5 m above the LiDAR surface at the selected point. By default, 1,440 horizontal rays are cast around the full circle, one every 0.25°. Buildings, vegetation and terrain can block them.</dd></div>
        <div><dt>Farthest sightline</dt><dd>The longest distance among rays that hit a surface. Rays that leave the dataset without a collision are excluded from this distance.</dd></div>
        <div><dt>Open directions</dt><dd>The percentage of all rays that leave the dataset without hitting a surface. This means no obstruction was found within coverage; it does not confirm visibility beyond it.</dd></div>
        <div><dt>Shaded view area</dt><dd>Blocked rays end at the surface they hit. On the map, open rays extend to the boundary of the LiDAR coverage. For area, water and landmark calculations, open rays are instead capped at the mean distance of blocked rays. The analysis polygon can therefore differ from the shaded map area.</dd></div>
        <div><dt>Sunrise & sunset</dt><dd>Times and compass bearings are computed for today in Vancouver time. The open percentage counts unobstructed horizontal rays within roughly 5.7° on either side of the sun’s bearing. It estimates directional openness, rather than simulating the sun’s elevation or confirming a visible sunrise or sunset.</dd></div>
        <div><dt>Ocean & landmarks</dt><dd>Ocean area is the overlap between the analysis polygon and ocean boundaries, measured in square metres and shown in square kilometres. Landmarks are named geometries intersecting that polygon. These overlaps do not independently verify that the water or landmark is visible.</dd></div>
        <div><dt>Elevation</dt><dd>Ground altitude is the LiDAR surface height at the selected point, which may be a roof or tree canopy. Eye altitude adds 2.5 m to that surface. Elevation and method are available in the sidebar’s expandable details.</dd></div>
        <div><dt>Panorama & AI description</dt><dd>The synthetic panorama covers 360° horizontally and −10° to +20° vertically, at four pixels per degree. Surface colors indicate LiDAR classes; distant surfaces fade. The AI describes four directional crops of this rendering. It can make mistakes, and synthetic sky colors do not establish weather, actual sky conditions or water.</dd></div>
      </dl>
    </section>

    <section aria-labelledby="help-score"><h2 id="help-score">Beauty score</h2>
      <p>An experimental composite of view area, water, landmarks and sun-direction openness. It reflects the project’s chosen weights rather than a universal measure of beauty.</p>
      <div className="help-table-wrap"><table className="help-score-table"><caption>Contributions to the beauty score</caption><thead><tr><th scope="col">Component</th><th scope="col">How points are assigned</th><th scope="col">Maximum</th></tr></thead><tbody>
        <tr><th scope="row">Openness</th><td>Analysis polygon area scales from 0 points at 1,000 m² to 75 points at 3,500,000 m².</td><td>75</td></tr>
        <tr><th scope="row">Water</th><td>Ocean plus lake overlap scales up to 1,750,000 m².</td><td>20</td></tr>
        <tr><th scope="row">Landmarks</th><td>2 points per intersecting landmark, up to five.</td><td>10</td></tr>
        <tr><th scope="row">Sunrise</th><td>The open fraction around the sunrise bearing × 10.</td><td>10</td></tr>
        <tr><th scope="row">Sunset</th><td>The open fraction around the sunset bearing × 10.</td><td>10</td></tr>
      </tbody></table></div>
      <p>Each contribution is capped at its maximum. The contributions are added, then the final score is capped at 100 and rounded for display. Openness here is polygon area; it differs from the sidebar’s Open directions percentage.</p>
    </section>

    <section aria-labelledby="help-coverage"><h2 id="help-coverage">Coverage & limitations</h2>
      <p>Results depend on the available LiDAR coverage and geographic layers. The grid is a surface approximation, not a live photograph or a complete 3D scene. Missing layers show as unavailable analysis rather than zero-valued metrics. Locations without LiDAR data, or with no blocked rays to establish the analysis boundary, cannot produce a view report.</p>
    </section>
    <button type="button" className="help-done" onClick={onClose}>Got it</button>
  </Modal>
}
